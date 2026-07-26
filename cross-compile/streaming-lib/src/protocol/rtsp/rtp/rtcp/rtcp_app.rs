use super::errors::RtcpError;
use super::rtcp_header::RtcpHeader;
use crate::io::bytes_reader::BytesReader;
use crate::io::bytes_writer::BytesWriter;
use crate::protocol::rtsp::rtp::utils::Marshal;
use crate::protocol::rtsp::rtp::utils::Unmarshal;
use byteorder::BigEndian;
use bytes::BytesMut;

//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |V=2|P|    ST   |   PT=APP=204  |             length            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                           SSRC/CSRC                           |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                          name (ASCII)                         |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                   application-dependent data                ...
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Debug, Clone, Default)]
pub struct RtcpApp {
    pub header: RtcpHeader,
    pub ssrc: u32,
    pub name: BytesMut,
    pub app_data: BytesMut,
}

impl Unmarshal<BytesMut, Result<Self, RtcpError>> for RtcpApp {
    fn unmarshal(data: BytesMut) -> Result<Self, RtcpError>
    where
        Self: Sized,
    {
        let mut reader = BytesReader::new(data);

        let header = RtcpHeader::unmarshal(&mut reader)?;
        let ssrc = reader.read_u32::<BigEndian>()?;
        let name = reader.read_bytes(4)?;
        let app_data_words = (header.length as usize)
            .checked_sub(2)
            .ok_or(RtcpError::from(
                super::errors::RtcpErrorValue::InvalidAppLength {
                    length: header.length,
                },
            ))?;
        let app_data_bytes = app_data_words * 4;
        let app_data = reader.read_bytes(app_data_bytes)?;

        Ok(RtcpApp {
            header,
            ssrc,
            name,
            app_data,
        })
    }
}

impl Marshal<Result<BytesMut, RtcpError>> for RtcpApp {
    fn marshal(&self) -> Result<BytesMut, RtcpError> {
        let mut writer = BytesWriter::default();

        let app_data_words = self.app_data.len().div_ceil(4);
        let mut header = self.header.clone();
        header.length = (2 + app_data_words) as u16;

        let header_bytesmut = header.marshal()?;
        writer.write(&header_bytesmut[..])?;

        writer.write_u32::<BigEndian>(self.ssrc)?;
        writer.write(&self.name[..])?;
        writer.write(&self.app_data[..])?;

        Ok(writer.extract_current_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Default Tests ==========

    #[test]
    fn test_rtcp_app_default() {
        let app = RtcpApp::default();
        assert_eq!(app.ssrc, 0);
        assert!(app.name.is_empty());
        assert!(app.app_data.is_empty());
    }

    // ========== Clone and Debug Tests ==========

    #[test]
    fn test_rtcp_app_clone() {
        let app = RtcpApp {
            ssrc: 0x12345678,
            name: BytesMut::from(&b"TEST"[..]),
            ..Default::default()
        };

        let cloned = app.clone();
        assert_eq!(cloned.ssrc, app.ssrc);
        assert_eq!(cloned.name, app.name);
    }

    #[test]
    fn test_rtcp_app_debug() {
        let app = RtcpApp::default();
        let debug_str = format!("{:?}", app);
        assert!(debug_str.contains("RtcpApp"));
        assert!(debug_str.contains("ssrc"));
    }

    // ========== Marshal Tests ==========

    #[test]
    fn test_rtcp_app_marshal_empty_data() {
        let app = RtcpApp {
            header: RtcpHeader {
                version: 2,
                padding_flag: 0,
                report_count: 0,
                payload_type: 204, // APP
                length: 2,         // 2 32-bit words after header
            },
            ssrc: 0x12345678,
            name: BytesMut::from(&b"TEST"[..]),
            app_data: BytesMut::new(),
        };

        let result = app.marshal().unwrap();
        // 4 (header) + 4 (SSRC) + 4 (name) = 12 bytes
        assert_eq!(result.len(), 12);
    }

    #[test]
    fn test_rtcp_app_marshal_with_data() {
        let app = RtcpApp {
            header: RtcpHeader {
                version: 2,
                padding_flag: 0,
                report_count: 0,
                payload_type: 204,
                length: 4,
            },
            ssrc: 0xAABBCCDD,
            name: BytesMut::from(&b"ABCD"[..]),
            app_data: BytesMut::from(&b"12345678"[..]),
        };

        let result = app.marshal().unwrap();
        // 4 (header) + 4 (SSRC) + 4 (name) + 8 (data) = 20 bytes
        assert_eq!(result.len(), 20);
    }

    // ========== Unmarshal Tests ==========

    #[test]
    fn test_rtcp_app_unmarshal_minimal() {
        // Header: version 2, pt 204, length 4
        // SSRC: 0x12345678
        // Name: "TEST"
        let mut data = BytesMut::new();
        data.extend_from_slice(&[0x80, 204, 0x00, 0x04]); // Header
        data.extend_from_slice(&[0x12, 0x34, 0x56, 0x78]); // SSRC
        data.extend_from_slice(b"TEST"); // Name
        // app_data: length=4 means 8 bytes of app data expected
        data.extend_from_slice(&[0; 8]); // Empty app data

        let app = RtcpApp::unmarshal(data).unwrap();
        assert_eq!(app.ssrc, 0x12345678);
        assert_eq!(&app.name[..], b"TEST");
    }

    // ========== Round-Trip Tests ==========

    #[test]
    fn test_rtcp_app_roundtrip() {
        let original = RtcpApp {
            header: RtcpHeader {
                version: 2,
                padding_flag: 0,
                report_count: 0,
                payload_type: 204,
                length: 2, // Will recalculate
            },
            ssrc: 0xDEADBEEF,
            name: BytesMut::from(&b"XYZ0"[..]),
            app_data: BytesMut::new(),
        };

        let marshaled = original.marshal().unwrap();
        // Add padding for length field (length = 0 means 0 extra bytes)
        let unmarshaled = RtcpApp::unmarshal(marshaled).unwrap();

        assert_eq!(unmarshaled.ssrc, original.ssrc);
        assert_eq!(unmarshaled.name, original.name);
    }
}
