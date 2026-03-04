use super::errors::RtcpError;
use super::rtcp_header::RtcpHeader;
use crate::io::bytes_reader::BytesReader;
use crate::io::bytes_writer::BytesWriter;
use crate::protocol::rtsp::rtp::utils::Marshal;
use crate::protocol::rtsp::rtp::utils::Unmarshal;
use byteorder::BigEndian;
use bytes::BytesMut;
//  	  0                   1                   2                   3
//  	  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// 	     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 	     |V=2|P|    SC   |   PT=BYE=203  |             length            |
// 	     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 	     |                           SSRC/CSRC                           |
// 	     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 	     :                              ...                              :
// 	     +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
// (opt) |     length    |            reason for leaving     ...
// 	     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

#[derive(Debug, Clone, Default)]
pub struct RtcpBye {
    pub header: RtcpHeader,
    pub ssrcs: Vec<u32>,
    pub length: u8,
    pub reason: BytesMut,
}

impl Unmarshal<BytesMut, Result<Self, RtcpError>> for RtcpBye {
    fn unmarshal(data: BytesMut) -> Result<Self, RtcpError>
    where
        Self: Sized,
    {
        let mut reader = BytesReader::new(data);

        let mut rtcp_bye = RtcpBye {
            header: RtcpHeader::unmarshal(&mut reader)?,
            ..Default::default()
        };

        for _ in 0..rtcp_bye.header.report_count {
            let ssrc = reader.read_u32::<BigEndian>()?;
            rtcp_bye.ssrcs.push(ssrc);
        }

        if rtcp_bye.header.length > 0 && !reader.is_empty() {
            rtcp_bye.length = reader.read_u8()?;
            if rtcp_bye.length > 0 && reader.len() >= rtcp_bye.length as usize {
                rtcp_bye.reason = reader.read_bytes(rtcp_bye.length as usize)?;
            }
        }

        Ok(rtcp_bye)
    }
}

impl Marshal<Result<BytesMut, RtcpError>> for RtcpBye {
    fn marshal(&self) -> Result<BytesMut, RtcpError> {
        let mut writer = BytesWriter::default();

        let header_bytesmut = self.header.marshal()?;
        writer.write(&header_bytesmut[..])?;

        for ssrc in &self.ssrcs {
            writer.write_u32::<BigEndian>(*ssrc)?;
        }

        writer.write_u8(self.length)?;
        writer.write(&self.reason[..])?;

        Ok(writer.extract_current_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Default Tests ==========

    #[test]
    fn test_rtcp_bye_default() {
        let bye = RtcpBye::default();
        assert!(bye.ssrcs.is_empty());
        assert_eq!(bye.length, 0);
        assert!(bye.reason.is_empty());
    }

    // ========== Clone and Debug Tests ==========

    #[test]
    fn test_rtcp_bye_clone() {
        let mut bye = RtcpBye::default();
        bye.ssrcs = vec![0x12345678, 0xAABBCCDD];
        bye.length = 5;
        bye.reason = BytesMut::from(&b"leave"[..]);

        let cloned = bye.clone();
        assert_eq!(cloned.ssrcs, bye.ssrcs);
        assert_eq!(cloned.length, bye.length);
        assert_eq!(cloned.reason, bye.reason);
    }

    #[test]
    fn test_rtcp_bye_debug() {
        let bye = RtcpBye::default();
        let debug_str = format!("{:?}", bye);
        assert!(debug_str.contains("RtcpBye"));
        assert!(debug_str.contains("ssrcs"));
    }

    // ========== Marshal Tests ==========

    #[test]
    fn test_rtcp_bye_marshal_no_ssrc() {
        let bye = RtcpBye {
            header: RtcpHeader {
                version: 2,
                padding_flag: 0,
                report_count: 0,
                payload_type: 203, // BYE
                length: 0,
            },
            ssrcs: vec![],
            length: 0,
            reason: BytesMut::new(),
        };

        let result = bye.marshal().unwrap();
        // 4 (header) + 0 (no SSRCs) + 1 (length) + 0 (no reason) = 5 bytes
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_rtcp_bye_marshal_single_ssrc() {
        let bye = RtcpBye {
            header: RtcpHeader {
                version: 2,
                padding_flag: 0,
                report_count: 1,
                payload_type: 203,
                length: 1,
            },
            ssrcs: vec![0x12345678],
            length: 0,
            reason: BytesMut::new(),
        };

        let result = bye.marshal().unwrap();
        // 4 (header) + 4 (1 SSRC) + 1 (length) + 0 (no reason) = 9 bytes
        assert_eq!(result.len(), 9);
    }

    #[test]
    fn test_rtcp_bye_marshal_with_reason() {
        let bye = RtcpBye {
            header: RtcpHeader {
                version: 2,
                padding_flag: 0,
                report_count: 1,
                payload_type: 203,
                length: 2,
            },
            ssrcs: vec![0xAABBCCDD],
            length: 7,
            reason: BytesMut::from(&b"leaving"[..]),
        };

        let result = bye.marshal().unwrap();
        // 4 (header) + 4 (1 SSRC) + 1 (length) + 7 (reason) = 16 bytes
        assert_eq!(result.len(), 16);
    }

    #[test]
    fn test_rtcp_bye_marshal_multiple_ssrcs() {
        let bye = RtcpBye {
            header: RtcpHeader {
                version: 2,
                padding_flag: 0,
                report_count: 3,
                payload_type: 203,
                length: 3,
            },
            ssrcs: vec![0x11111111, 0x22222222, 0x33333333],
            length: 0,
            reason: BytesMut::new(),
        };

        let result = bye.marshal().unwrap();
        // 4 (header) + 12 (3 SSRCs) + 1 (length) + 0 (no reason) = 17 bytes
        assert_eq!(result.len(), 17);
    }

    // ========== Unmarshal Tests ==========

    #[test]
    fn test_rtcp_bye_unmarshal_single_ssrc() {
        let mut data = BytesMut::new();
        // Header: version 2, report_count 1, pt 203
        data.extend_from_slice(&[0x81, 203, 0x00, 0x01]); // Header
        data.extend_from_slice(&[0x12, 0x34, 0x56, 0x78]); // SSRC
        data.extend_from_slice(&[0x00]); // length 0 (no reason)

        let bye = RtcpBye::unmarshal(data).unwrap();
        assert_eq!(bye.ssrcs.len(), 1);
        assert_eq!(bye.ssrcs[0], 0x12345678);
        assert_eq!(bye.length, 0);
    }

    #[test]
    fn test_rtcp_bye_unmarshal_with_reason() {
        let mut data = BytesMut::new();
        data.extend_from_slice(&[0x81, 203, 0x00, 0x02]); // Header
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]); // SSRC
        data.extend_from_slice(&[0x03]); // length 3
        data.extend_from_slice(b"bye"); // reason

        let bye = RtcpBye::unmarshal(data).unwrap();
        assert_eq!(bye.ssrcs[0], 0xAABBCCDD);
        assert_eq!(bye.length, 3);
        assert_eq!(&bye.reason[..], b"bye");
    }

    // ========== Round-Trip Tests ==========

    #[test]
    fn test_rtcp_bye_roundtrip_no_reason() {
        let original = RtcpBye {
            header: RtcpHeader {
                version: 2,
                padding_flag: 0,
                report_count: 1,
                payload_type: 203,
                length: 1,
            },
            ssrcs: vec![0xDEADBEEF],
            length: 0,
            reason: BytesMut::new(),
        };

        let marshaled = original.marshal().unwrap();
        let unmarshaled = RtcpBye::unmarshal(marshaled).unwrap();

        assert_eq!(unmarshaled.ssrcs, original.ssrcs);
        assert_eq!(unmarshaled.length, original.length);
    }

    #[test]
    fn test_rtcp_bye_roundtrip_with_reason() {
        let original = RtcpBye {
            header: RtcpHeader {
                version: 2,
                padding_flag: 0,
                report_count: 2,
                payload_type: 203,
                length: 3,
            },
            ssrcs: vec![0x11111111, 0x22222222],
            length: 4,
            reason: BytesMut::from(&b"quit"[..]),
        };

        let marshaled = original.marshal().unwrap();
        let unmarshaled = RtcpBye::unmarshal(marshaled).unwrap();

        assert_eq!(unmarshaled.ssrcs, original.ssrcs);
        assert_eq!(unmarshaled.length, original.length);
        assert_eq!(unmarshaled.reason, original.reason);
    }
}
