use crate::io::bytes_errors::BytesReadError;
use crate::io::bytes_errors::BytesWriteError;
use crate::io::bytes_reader::BytesReader;
use crate::io::bytes_writer::BytesWriter;
use byteorder::BigEndian;
use bytes::BytesMut;

use super::define::RTP_FIXED_HEADER_LEN;
use super::utils::Marshal;
use super::utils::Unmarshal;

#[derive(Debug, Clone)]
pub struct RtpHeader {
    pub version: u8,        // 2 bits
    pub padding_flag: u8,   // 1 bit
    pub extension_flag: u8, // 1 bit
    pub cc: u8,             // 4 bits
    pub marker: u8,         // 1 bit
    pub payload_type: u8,   // 7 bits
    pub seq_number: u16,
    pub timestamp: u32,
    pub ssrc: u32,
    pub csrcs: Vec<u32>,
}

/// RFC 3550 §5.1 mandates RTP version 2.
impl Default for RtpHeader {
    fn default() -> Self {
        Self {
            version: 2,
            padding_flag: 0,
            extension_flag: 0,
            cc: 0,
            marker: 0,
            payload_type: 0,
            seq_number: 0,
            timestamp: 0,
            ssrc: 0,
            csrcs: Vec::new(),
        }
    }
}

impl Unmarshal<&mut BytesReader, Result<Self, BytesReadError>> for RtpHeader {
    fn unmarshal(reader: &mut BytesReader) -> Result<Self, BytesReadError>
    where
        Self: Sized,
    {
        let mut rtp_header = RtpHeader::default();

        let byte_1st: u8 = reader.read_u8()?;
        rtp_header.version = byte_1st >> 6;
        rtp_header.padding_flag = byte_1st >> 5 & 0x01;
        rtp_header.extension_flag = byte_1st >> 4 & 0x01;
        rtp_header.cc = byte_1st & 0x0F;

        let byte_2nd = reader.read_u8()?;
        rtp_header.marker = byte_2nd >> 7;
        rtp_header.payload_type = byte_2nd & 0x7F;
        rtp_header.seq_number = reader.read_u16::<BigEndian>()?;
        rtp_header.timestamp = reader.read_u32::<BigEndian>()?;
        rtp_header.ssrc = reader.read_u32::<BigEndian>()?;

        for _ in 0..rtp_header.cc {
            rtp_header.csrcs.push(reader.read_u32::<BigEndian>()?);
        }

        Ok(rtp_header)
    }
}

impl Marshal<Result<BytesMut, BytesWriteError>> for RtpHeader {
    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |V=2|P|X|  CC   |M|     PT      |       sequence number         |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                           timestamp                           |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |           synchronization source (SSRC) identifier            |
    // +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
    // |            contributing source (CSRC) identifiers             |
    // |                             ....                              |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    fn marshal(&self) -> Result<BytesMut, BytesWriteError> {
        let mut writer = BytesWriter::with_capacity(self.marshalled_len());
        self.marshal_into(&mut writer)?;
        Ok(writer.extract_current_bytes())
    }
}

impl RtpHeader {
    /// Serialised size in bytes: the 12-byte fixed header plus one word per CSRC.
    pub fn marshalled_len(&self) -> usize {
        RTP_FIXED_HEADER_LEN + self.csrcs.len() * 4
    }

    /// Write this header into an existing writer.
    ///
    /// [`Marshal::marshal`] allocates a fresh buffer for what is usually 12 bytes. On the send path
    /// that is one allocation per datagram whose only purpose is to be copied into the packet's
    /// buffer and dropped, so [`RtpPacket::marshal`] writes the header straight into its own.
    pub fn marshal_into(&self, writer: &mut BytesWriter) -> Result<(), BytesWriteError> {
        let csrc_count = (self.csrcs.len() as u8) & 0x0F;
        let byte_1st: u8 = (self.version << 6)
            | (self.padding_flag << 5)
            | (self.extension_flag << 4)
            | csrc_count;
        writer.write_u8(byte_1st)?;

        let byte_2nd: u8 = (self.marker << 7) | self.payload_type;
        writer.write_u8(byte_2nd)?;

        writer.write_u16::<BigEndian>(self.seq_number)?;
        writer.write_u32::<BigEndian>(self.timestamp)?;
        writer.write_u32::<BigEndian>(self.ssrc)?;

        for csrc in &self.csrcs {
            writer.write_u32::<BigEndian>(*csrc)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    // ===========================================
    // Construction and Default Tests
    // ===========================================

    #[test]
    fn test_default_rtp_header() {
        let header = RtpHeader::default();
        assert_eq!(header.version, 2);
        assert_eq!(header.padding_flag, 0);
        assert_eq!(header.extension_flag, 0);
        assert_eq!(header.cc, 0);
        assert_eq!(header.marker, 0);
        assert_eq!(header.payload_type, 0);
        assert_eq!(header.seq_number, 0);
        assert_eq!(header.timestamp, 0);
        assert_eq!(header.ssrc, 0);
        assert!(header.csrcs.is_empty());
    }

    #[test]
    fn test_clone_rtp_header() {
        let header = RtpHeader {
            version: 2,
            padding_flag: 1,
            extension_flag: 0,
            cc: 3,
            marker: 1,
            payload_type: 96,
            seq_number: 12345,
            timestamp: 90000,
            ssrc: 0x12345678,
            csrcs: vec![0xAABBCCDD, 0x11223344, 0x55667788],
        };
        let cloned = header.clone();
        assert_eq!(header.version, cloned.version);
        assert_eq!(header.padding_flag, cloned.padding_flag);
        assert_eq!(header.extension_flag, cloned.extension_flag);
        assert_eq!(header.cc, cloned.cc);
        assert_eq!(header.marker, cloned.marker);
        assert_eq!(header.payload_type, cloned.payload_type);
        assert_eq!(header.seq_number, cloned.seq_number);
        assert_eq!(header.timestamp, cloned.timestamp);
        assert_eq!(header.ssrc, cloned.ssrc);
        assert_eq!(header.csrcs, cloned.csrcs);
    }

    // ===========================================
    // Marshal Tests - Bit Field Packing
    // ===========================================

    #[test]
    fn test_marshal_minimal_header() {
        let header = RtpHeader {
            version: 2,
            padding_flag: 0,
            extension_flag: 0,
            cc: 0,
            marker: 0,
            payload_type: 0,
            seq_number: 0,
            timestamp: 0,
            ssrc: 0,
            csrcs: vec![],
        };
        let result = header.marshal().unwrap();
        assert_eq!(result.len(), 12); // Minimum RTP header size
        assert_eq!(result[0], 0x80); // Version 2 in first 2 bits
        assert_eq!(result[1], 0x00); // Marker 0, PT 0
    }

    #[test]
    fn test_marshal_version_field() {
        // Version 2 (standard RTP)
        let header = RtpHeader {
            version: 2,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[0] >> 6, 2);

        // Version 0
        let header = RtpHeader {
            version: 0,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[0] >> 6, 0);

        // Version 3 (max value for 2 bits)
        let header = RtpHeader {
            version: 3,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[0] >> 6, 3);
    }

    #[test]
    fn test_marshal_padding_flag() {
        let header = RtpHeader {
            version: 2,
            padding_flag: 1,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!((result[0] >> 5) & 0x01, 1);

        let header = RtpHeader {
            version: 2,
            padding_flag: 0,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!((result[0] >> 5) & 0x01, 0);
    }

    #[test]
    fn test_marshal_extension_flag() {
        let header = RtpHeader {
            version: 2,
            extension_flag: 1,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!((result[0] >> 4) & 0x01, 1);

        let header = RtpHeader {
            version: 2,
            extension_flag: 0,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!((result[0] >> 4) & 0x01, 0);
    }

    #[test]
    fn test_marshal_cc_field() {
        // CC = 0
        let header = RtpHeader {
            version: 2,
            cc: 0,
            csrcs: vec![],
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[0] & 0x0F, 0);
        assert_eq!(result.len(), 12);

        // CC = 5
        let header = RtpHeader {
            version: 2,
            cc: 5,
            csrcs: vec![1, 2, 3, 4, 5],
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[0] & 0x0F, 5);
        assert_eq!(result.len(), 12 + 5 * 4);

        // CC = 15 (max value for 4 bits)
        let header = RtpHeader {
            version: 2,
            cc: 15,
            csrcs: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[0] & 0x0F, 15);
        assert_eq!(result.len(), 12 + 15 * 4);
    }

    #[test]
    fn test_marshal_marker_bit() {
        let header = RtpHeader {
            version: 2,
            marker: 1,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[1] >> 7, 1);

        let header = RtpHeader {
            version: 2,
            marker: 0,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[1] >> 7, 0);
    }

    #[test]
    fn test_marshal_payload_type() {
        // PT = 0 (PCMU)
        let header = RtpHeader {
            version: 2,
            payload_type: 0,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[1] & 0x7F, 0);

        // PT = 96 (dynamic range start)
        let header = RtpHeader {
            version: 2,
            payload_type: 96,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[1] & 0x7F, 96);

        // PT = 127 (max value for 7 bits)
        let header = RtpHeader {
            version: 2,
            payload_type: 127,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[1] & 0x7F, 127);
    }

    #[test]
    fn test_marshal_sequence_number() {
        let header = RtpHeader {
            version: 2,
            seq_number: 0,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(&result[2..4], &[0x00, 0x00]);

        let header = RtpHeader {
            version: 2,
            seq_number: 0x1234,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(&result[2..4], &[0x12, 0x34]);

        let header = RtpHeader {
            version: 2,
            seq_number: 0xFFFF,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(&result[2..4], &[0xFF, 0xFF]);
    }

    #[test]
    fn test_marshal_timestamp() {
        let header = RtpHeader {
            version: 2,
            timestamp: 0,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(&result[4..8], &[0x00, 0x00, 0x00, 0x00]);

        let header = RtpHeader {
            version: 2,
            timestamp: 0x12345678,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(&result[4..8], &[0x12, 0x34, 0x56, 0x78]);

        let header = RtpHeader {
            version: 2,
            timestamp: 0xFFFFFFFF,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(&result[4..8], &[0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn test_marshal_ssrc() {
        let header = RtpHeader {
            version: 2,
            ssrc: 0,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(&result[8..12], &[0x00, 0x00, 0x00, 0x00]);

        let header = RtpHeader {
            version: 2,
            ssrc: 0xDEADBEEF,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(&result[8..12], &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_marshal_csrc_list() {
        let header = RtpHeader {
            version: 2,
            cc: 3,
            csrcs: vec![0x11111111, 0x22222222, 0x33333333],
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result.len(), 12 + 12); // 12 base + 3*4 CSRCs
        assert_eq!(&result[12..16], &[0x11, 0x11, 0x11, 0x11]);
        assert_eq!(&result[16..20], &[0x22, 0x22, 0x22, 0x22]);
        assert_eq!(&result[20..24], &[0x33, 0x33, 0x33, 0x33]);
    }

    #[test]
    fn test_marshal_combined_first_byte() {
        // V=2, P=1, X=1, CC=7 should give: 10 1 1 0111 = 0xB7
        let header = RtpHeader {
            version: 2,
            padding_flag: 1,
            extension_flag: 1,
            cc: 7,
            csrcs: vec![1, 2, 3, 4, 5, 6, 7],
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[0], 0xB7);
    }

    #[test]
    fn test_marshal_combined_second_byte() {
        // M=1, PT=96 should give: 1 1100000 = 0xE0
        let header = RtpHeader {
            version: 2,
            marker: 1,
            payload_type: 96,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[1], 0xE0);
    }

    // ===========================================
    // Unmarshal Tests - Bit Field Unpacking
    // ===========================================

    #[test]
    fn test_unmarshal_minimal_header() {
        let data = BytesMut::from(
            &[
                0x80, // V=2, P=0, X=0, CC=0
                0x00, // M=0, PT=0
                0x00, 0x00, // seq=0
                0x00, 0x00, 0x00, 0x00, // timestamp=0
                0x00, 0x00, 0x00, 0x00, // ssrc=0
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.version, 2);
        assert_eq!(header.padding_flag, 0);
        assert_eq!(header.extension_flag, 0);
        assert_eq!(header.cc, 0);
        assert_eq!(header.marker, 0);
        assert_eq!(header.payload_type, 0);
        assert_eq!(header.seq_number, 0);
        assert_eq!(header.timestamp, 0);
        assert_eq!(header.ssrc, 0);
        assert!(header.csrcs.is_empty());
    }

    #[test]
    fn test_unmarshal_version_field() {
        // Version 0
        let data = BytesMut::from(
            &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.version, 0);

        // Version 2
        let data = BytesMut::from(
            &[
                0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.version, 2);

        // Version 3
        let data = BytesMut::from(
            &[
                0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.version, 3);
    }

    #[test]
    fn test_unmarshal_padding_flag() {
        // P=1
        let data = BytesMut::from(
            &[
                0xA0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.version, 2);
        assert_eq!(header.padding_flag, 1);
    }

    #[test]
    fn test_unmarshal_extension_flag() {
        // X=1
        let data = BytesMut::from(
            &[
                0x90, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.version, 2);
        assert_eq!(header.extension_flag, 1);
    }

    #[test]
    fn test_unmarshal_cc_field() {
        // CC=5 with 5 CSRCs
        let data = BytesMut::from(
            &[
                0x85, // V=2, CC=5
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x01, // CSRC 1
                0x00, 0x00, 0x00, 0x02, // CSRC 2
                0x00, 0x00, 0x00, 0x03, // CSRC 3
                0x00, 0x00, 0x00, 0x04, // CSRC 4
                0x00, 0x00, 0x00, 0x05, // CSRC 5
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.cc, 5);
        assert_eq!(header.csrcs.len(), 5);
        assert_eq!(header.csrcs, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_unmarshal_marker_bit() {
        // M=1
        let data = BytesMut::from(
            &[
                0x80, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.marker, 1);
    }

    #[test]
    fn test_unmarshal_payload_type() {
        // PT=96
        let data = BytesMut::from(
            &[
                0x80, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.payload_type, 96);

        // PT=127
        let data = BytesMut::from(
            &[
                0x80, 0x7F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.payload_type, 127);
    }

    #[test]
    fn test_unmarshal_sequence_number() {
        let data = BytesMut::from(
            &[
                0x80, 0x00, 0x12, 0x34, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.seq_number, 0x1234);
    }

    #[test]
    fn test_unmarshal_timestamp() {
        let data = BytesMut::from(
            &[
                0x80, 0x00, 0x00, 0x00, 0x12, 0x34, 0x56, 0x78, 0x00, 0x00, 0x00, 0x00,
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.timestamp, 0x12345678);
    }

    #[test]
    fn test_unmarshal_ssrc() {
        let data = BytesMut::from(
            &[
                0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF,
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.ssrc, 0xDEADBEEF);
    }

    #[test]
    fn test_unmarshal_combined_first_byte() {
        // V=2, P=1, X=1, CC=7 = 0xB7
        let data = BytesMut::from(
            &[
                0xB7, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01,
                0x01, 0x01, // CSRC 1
                0x02, 0x02, 0x02, 0x02, // CSRC 2
                0x03, 0x03, 0x03, 0x03, // CSRC 3
                0x04, 0x04, 0x04, 0x04, // CSRC 4
                0x05, 0x05, 0x05, 0x05, // CSRC 5
                0x06, 0x06, 0x06, 0x06, // CSRC 6
                0x07, 0x07, 0x07, 0x07, // CSRC 7
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.version, 2);
        assert_eq!(header.padding_flag, 1);
        assert_eq!(header.extension_flag, 1);
        assert_eq!(header.cc, 7);
        assert_eq!(header.csrcs.len(), 7);
    }

    #[test]
    fn test_unmarshal_combined_second_byte() {
        // M=1, PT=96 = 0xE0
        let data = BytesMut::from(
            &[
                0x80, 0xE0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.marker, 1);
        assert_eq!(header.payload_type, 96);
    }

    // ===========================================
    // Marshal/Unmarshal Roundtrip Tests
    // ===========================================

    #[test]
    fn test_roundtrip_minimal_header() {
        let original = RtpHeader {
            version: 2,
            padding_flag: 0,
            extension_flag: 0,
            cc: 0,
            marker: 0,
            payload_type: 0,
            seq_number: 0,
            timestamp: 0,
            ssrc: 0,
            csrcs: vec![],
        };
        let marshaled = original.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtpHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(original.version, unmarshaled.version);
        assert_eq!(original.padding_flag, unmarshaled.padding_flag);
        assert_eq!(original.extension_flag, unmarshaled.extension_flag);
        assert_eq!(original.cc, unmarshaled.cc);
        assert_eq!(original.marker, unmarshaled.marker);
        assert_eq!(original.payload_type, unmarshaled.payload_type);
        assert_eq!(original.seq_number, unmarshaled.seq_number);
        assert_eq!(original.timestamp, unmarshaled.timestamp);
        assert_eq!(original.ssrc, unmarshaled.ssrc);
        assert_eq!(original.csrcs, unmarshaled.csrcs);
    }

    #[test]
    fn test_roundtrip_full_header() {
        let original = RtpHeader {
            version: 2,
            padding_flag: 1,
            extension_flag: 1,
            cc: 5,
            marker: 1,
            payload_type: 96,
            seq_number: 54321,
            timestamp: 1234567890,
            ssrc: 0xABCDEF01,
            csrcs: vec![0x11111111, 0x22222222, 0x33333333, 0x44444444, 0x55555555],
        };
        let marshaled = original.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtpHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(original.version, unmarshaled.version);
        assert_eq!(original.padding_flag, unmarshaled.padding_flag);
        assert_eq!(original.extension_flag, unmarshaled.extension_flag);
        assert_eq!(original.cc, unmarshaled.cc);
        assert_eq!(original.marker, unmarshaled.marker);
        assert_eq!(original.payload_type, unmarshaled.payload_type);
        assert_eq!(original.seq_number, unmarshaled.seq_number);
        assert_eq!(original.timestamp, unmarshaled.timestamp);
        assert_eq!(original.ssrc, unmarshaled.ssrc);
        assert_eq!(original.csrcs, unmarshaled.csrcs);
    }

    #[test]
    fn test_roundtrip_max_csrc() {
        let original = RtpHeader {
            version: 2,
            padding_flag: 0,
            extension_flag: 0,
            cc: 15,
            marker: 0,
            payload_type: 0,
            seq_number: 0,
            timestamp: 0,
            ssrc: 0,
            csrcs: (1..=15).map(|i| i as u32 * 0x01010101).collect(),
        };
        let marshaled = original.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtpHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(original.cc, unmarshaled.cc);
        assert_eq!(original.csrcs.len(), 15);
        assert_eq!(original.csrcs, unmarshaled.csrcs);
    }

    #[test]
    fn test_roundtrip_max_values() {
        let original = RtpHeader {
            version: 3,
            padding_flag: 1,
            extension_flag: 1,
            cc: 15,
            marker: 1,
            payload_type: 127,
            seq_number: 0xFFFF,
            timestamp: 0xFFFFFFFF,
            ssrc: 0xFFFFFFFF,
            csrcs: vec![0xFFFFFFFF; 15],
        };
        let marshaled = original.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtpHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(original.version, unmarshaled.version);
        assert_eq!(original.padding_flag, unmarshaled.padding_flag);
        assert_eq!(original.extension_flag, unmarshaled.extension_flag);
        assert_eq!(original.cc, unmarshaled.cc);
        assert_eq!(original.marker, unmarshaled.marker);
        assert_eq!(original.payload_type, unmarshaled.payload_type);
        assert_eq!(original.seq_number, unmarshaled.seq_number);
        assert_eq!(original.timestamp, unmarshaled.timestamp);
        assert_eq!(original.ssrc, unmarshaled.ssrc);
        assert_eq!(original.csrcs, unmarshaled.csrcs);
    }

    #[test]
    fn test_roundtrip_typical_h264_header() {
        // Typical H.264 video RTP header
        let original = RtpHeader {
            version: 2,
            padding_flag: 0,
            extension_flag: 0,
            cc: 0,
            marker: 1,        // End of frame
            payload_type: 96, // Dynamic PT for H.264
            seq_number: 12345,
            timestamp: 90000, // 90kHz clock
            ssrc: 0x12345678,
            csrcs: vec![],
        };
        let marshaled = original.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtpHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(original.version, unmarshaled.version);
        assert_eq!(original.marker, unmarshaled.marker);
        assert_eq!(original.payload_type, unmarshaled.payload_type);
        assert_eq!(original.seq_number, unmarshaled.seq_number);
        assert_eq!(original.timestamp, unmarshaled.timestamp);
        assert_eq!(original.ssrc, unmarshaled.ssrc);
    }

    #[test]
    fn test_roundtrip_audio_header() {
        // Typical audio RTP header
        let original = RtpHeader {
            version: 2,
            padding_flag: 0,
            extension_flag: 0,
            cc: 0,
            marker: 0,
            payload_type: 0, // PCMU
            seq_number: 1000,
            timestamp: 8000, // 8kHz clock
            ssrc: 0x87654321,
            csrcs: vec![],
        };
        let marshaled = original.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtpHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(original.payload_type, unmarshaled.payload_type);
        assert_eq!(original.timestamp, unmarshaled.timestamp);
    }

    // ===========================================
    // CSRC List Handling Tests
    // ===========================================

    #[test]
    fn test_csrc_list_empty() {
        let header = RtpHeader {
            version: 2,
            cc: 0,
            csrcs: vec![],
            ..Default::default()
        };
        let marshaled = header.marshal().unwrap();
        assert_eq!(marshaled.len(), 12);
    }

    #[test]
    fn test_csrc_list_single() {
        let header = RtpHeader {
            version: 2,
            cc: 1,
            csrcs: vec![0x11223344],
            ..Default::default()
        };
        let marshaled = header.marshal().unwrap();
        assert_eq!(marshaled.len(), 16);

        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(unmarshaled.csrcs.len(), 1);
        assert_eq!(unmarshaled.csrcs[0], 0x11223344);
    }

    #[test]
    fn test_csrc_list_max() {
        let csrcs: Vec<u32> = (0..15).map(|i| i * 0x10101010 + 0x01010101).collect();
        let header = RtpHeader {
            version: 2,
            cc: 15,
            csrcs: csrcs.clone(),
            ..Default::default()
        };
        let marshaled = header.marshal().unwrap();
        assert_eq!(marshaled.len(), 12 + 15 * 4);

        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(unmarshaled.csrcs.len(), 15);
        assert_eq!(unmarshaled.csrcs, csrcs);
    }

    #[test]
    fn test_csrc_boundary_values() {
        let header = RtpHeader {
            version: 2,
            cc: 4,
            csrcs: vec![0x00000000, 0x00000001, 0x7FFFFFFF, 0xFFFFFFFF],
            ..Default::default()
        };
        let marshaled = header.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtpHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(unmarshaled.csrcs[0], 0x00000000);
        assert_eq!(unmarshaled.csrcs[1], 0x00000001);
        assert_eq!(unmarshaled.csrcs[2], 0x7FFFFFFF);
        assert_eq!(unmarshaled.csrcs[3], 0xFFFFFFFF);
    }

    // ===========================================
    // Sequence Number and Timestamp Tests
    // ===========================================

    #[test]
    fn test_sequence_number_wraparound() {
        let header = RtpHeader {
            version: 2,
            seq_number: 0xFFFF,
            ..Default::default()
        };
        let marshaled = header.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(unmarshaled.seq_number, 0xFFFF);

        // Simulate wraparound
        let header = RtpHeader {
            version: 2,
            seq_number: 0,
            ..Default::default()
        };
        let marshaled = header.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(unmarshaled.seq_number, 0);
    }

    #[test]
    fn test_timestamp_90khz_values() {
        // 90kHz clock typical values
        let timestamps = [0u32, 90000, 180000, 270000, 3600000, 0xFFFFFFFF];
        for ts in timestamps {
            let header = RtpHeader {
                version: 2,
                timestamp: ts,
                ..Default::default()
            };
            let marshaled = header.marshal().unwrap();
            let mut reader = BytesReader::new(marshaled);
            let unmarshaled = RtpHeader::unmarshal(&mut reader).unwrap();
            assert_eq!(unmarshaled.timestamp, ts);
        }
    }

    // ===========================================
    // SSRC Validation Tests
    // ===========================================

    #[test]
    fn test_ssrc_zero() {
        let header = RtpHeader {
            version: 2,
            ssrc: 0,
            ..Default::default()
        };
        let marshaled = header.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(unmarshaled.ssrc, 0);
    }

    #[test]
    fn test_ssrc_max() {
        let header = RtpHeader {
            version: 2,
            ssrc: 0xFFFFFFFF,
            ..Default::default()
        };
        let marshaled = header.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(unmarshaled.ssrc, 0xFFFFFFFF);
    }

    #[test]
    fn test_ssrc_random_values() {
        let ssrc_values = [0x12345678u32, 0xDEADBEEF, 0xCAFEBABE, 0x01234567];
        for ssrc in ssrc_values {
            let header = RtpHeader {
                version: 2,
                ssrc,
                ..Default::default()
            };
            let marshaled = header.marshal().unwrap();
            let mut reader = BytesReader::new(marshaled);
            let unmarshaled = RtpHeader::unmarshal(&mut reader).unwrap();
            assert_eq!(unmarshaled.ssrc, ssrc);
        }
    }

    // ===========================================
    // Error Condition Tests
    // ===========================================

    #[test]
    fn test_unmarshal_insufficient_data() {
        // Less than 12 bytes
        let data = BytesMut::from(&[0x80, 0x00, 0x00][..]);
        let mut reader = BytesReader::new(data);
        let result = RtpHeader::unmarshal(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_unmarshal_missing_csrc_data() {
        // Header says CC=5 but no CSRC data
        let data = BytesMut::from(
            &[
                0x85, // V=2, CC=5
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00,
                // Missing 5 CSRCs
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let result = RtpHeader::unmarshal(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_unmarshal_partial_csrc_data() {
        // Header says CC=3 but only has 2 CSRCs
        let data = BytesMut::from(
            &[
                0x83, // V=2, CC=3
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x01, // CSRC 1
                0x00, 0x00, 0x00, 0x02, // CSRC 2
                      // Missing CSRC 3
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let result = RtpHeader::unmarshal(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_unmarshal_empty_data() {
        let data = BytesMut::new();
        let mut reader = BytesReader::new(data);
        let result = RtpHeader::unmarshal(&mut reader);
        assert!(result.is_err());
    }

    // ===========================================
    // Header Size Tests
    // ===========================================

    #[test]
    fn test_marshal_header_size_no_csrc() {
        let header = RtpHeader {
            version: 2,
            cc: 0,
            csrcs: vec![],
            ..Default::default()
        };
        let marshaled = header.marshal().unwrap();
        assert_eq!(marshaled.len(), 12);
    }

    #[test]
    fn test_marshal_header_size_with_csrcs() {
        for cc in 1..=15 {
            let csrcs: Vec<u32> = (0..cc).map(|_| 0).collect();
            let header = RtpHeader {
                version: 2,
                cc: cc as u8,
                csrcs,
                ..Default::default()
            };
            let marshaled = header.marshal().unwrap();
            assert_eq!(marshaled.len(), 12 + (cc * 4));
        }
    }

    // ===========================================
    // Real-World Packet Tests
    // ===========================================

    #[test]
    fn test_real_world_h264_packet() {
        // Real H.264 RTP packet header (captured)
        let data = BytesMut::from(
            &[
                0x80, // V=2, P=0, X=0, CC=0
                0xE0, // M=1, PT=96
                0x00, 0x01, // seq=1
                0x00, 0x00, 0x00, 0x00, // timestamp=0
                0x12, 0x34, 0x56, 0x78, // ssrc
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(header.version, 2);
        assert_eq!(header.padding_flag, 0);
        assert_eq!(header.extension_flag, 0);
        assert_eq!(header.cc, 0);
        assert_eq!(header.marker, 1);
        assert_eq!(header.payload_type, 96);
        assert_eq!(header.seq_number, 1);
        assert_eq!(header.timestamp, 0);
        assert_eq!(header.ssrc, 0x12345678);
    }

    #[test]
    fn test_real_world_audio_packet() {
        // Real audio RTP packet header (PCMU)
        let data = BytesMut::from(
            &[
                0x80, // V=2, P=0, X=0, CC=0
                0x00, // M=0, PT=0 (PCMU)
                0x03, 0xE8, // seq=1000
                0x00, 0x01, 0xF4, 0x00, // timestamp=128000
                0x87, 0x65, 0x43, 0x21, // ssrc
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(header.version, 2);
        assert_eq!(header.marker, 0);
        assert_eq!(header.payload_type, 0);
        assert_eq!(header.seq_number, 1000);
        assert_eq!(header.timestamp, 128000);
        assert_eq!(header.ssrc, 0x87654321);
    }

    #[test]
    fn test_mixer_packet_with_csrcs() {
        // RTP packet from a mixer with contributing sources
        let data = BytesMut::from(
            &[
                0x84, // V=2, P=0, X=0, CC=4
                0x60, // M=0, PT=96
                0x00, 0x64, // seq=100
                0x00, 0x01, 0x51, 0x80, // timestamp=86400
                0xAB, 0xCD, 0xEF, 0x01, // ssrc
                0x11, 0x11, 0x11, 0x11, // CSRC 1
                0x22, 0x22, 0x22, 0x22, // CSRC 2
                0x33, 0x33, 0x33, 0x33, // CSRC 3
                0x44, 0x44, 0x44, 0x44, // CSRC 4
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let header = RtpHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(header.version, 2);
        assert_eq!(header.cc, 4);
        assert_eq!(header.payload_type, 96);
        assert_eq!(header.seq_number, 100);
        assert_eq!(header.timestamp, 86400);
        assert_eq!(header.ssrc, 0xABCDEF01);
        assert_eq!(header.csrcs.len(), 4);
        assert_eq!(header.csrcs[0], 0x11111111);
        assert_eq!(header.csrcs[1], 0x22222222);
        assert_eq!(header.csrcs[2], 0x33333333);
        assert_eq!(header.csrcs[3], 0x44444444);
    }
}
