pub mod define;
pub mod errors;
pub mod rtcp;
pub mod rtp_aac;
pub mod rtp_h264;
pub mod rtp_header;
pub mod utils;

use crate::io::bytes_errors::BytesReadError;
use crate::io::bytes_errors::BytesWriteError;
use crate::io::bytes_reader::BytesReader;
use crate::io::bytes_writer::BytesWriter;
use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};
use rtp_header::RtpHeader;

use self::utils::Marshal;
use self::utils::Unmarshal;

#[derive(Debug, Clone, Default)]
pub struct RtpPacket {
    pub header: RtpHeader,
    pub header_extension_profile: u16,
    pub header_extension_length: u16,
    pub header_extension_payload: BytesMut,
    pub payload: BytesMut,
    pub padding: BytesMut,
}

impl Unmarshal<&mut BytesReader, Result<Self, BytesReadError>> for RtpPacket {
    //https://blog.jianchihu.net/webrtc-research-rtp-header-extension.html
    fn unmarshal(reader: &mut BytesReader) -> Result<Self, BytesReadError>
    where
        Self: Sized,
    {
        let mut rtp_packet = RtpPacket {
            header: RtpHeader::unmarshal(reader)?,
            ..Default::default()
        };

        if rtp_packet.header.extension_flag == 1 {
            // 0                   1                   2                   3
            // 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
            // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
            // |      defined by profile       |           length              |
            // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
            // |                        header extension                       |
            // |                             ....                              |
            // header_extension = profile(2 bytes) + length(2 bytes) + header extension payload
            rtp_packet.header_extension_profile = reader.read_u16::<BigEndian>()?;
            rtp_packet.header_extension_length = reader.read_u16::<BigEndian>()?;
            rtp_packet.header_extension_payload =
                reader.read_bytes(4 * rtp_packet.header_extension_length as usize)?;
        }

        if rtp_packet.header.padding_flag == 1 {
            let padding_length = reader.get(reader.len() - 1)? as usize;
            rtp_packet
                .payload
                .put(reader.read_bytes(reader.len() - padding_length)?);
            rtp_packet.padding.put(reader.read_bytes(padding_length)?);
        } else {
            rtp_packet.payload.put(reader.extract_remaining_bytes());
        }

        Ok(rtp_packet)
    }
}

impl RtpPacket {
    /// Exact serialised size, so [`Marshal::marshal`] can allocate once.
    ///
    /// Kept next to `marshal` deliberately: the two must agree, and a mismatch is silent -- too
    /// small only costs a reallocation, so it would never show up as a failure.
    fn marshalled_len(&self) -> usize {
        let extension = if self.header.extension_flag == 1 {
            4 + self.header_extension_payload.len()
        } else {
            0
        };
        let padding = if self.header.padding_flag == 1 {
            self.padding.len()
        } else {
            0
        };
        self.header.marshalled_len() + extension + self.payload.len() + padding
    }
}

impl Marshal<Result<BytesMut, BytesWriteError>> for RtpPacket {
    fn marshal(&self) -> Result<BytesMut, BytesWriteError> {
        // Sized up front and the header written in place. Growing from empty reallocated and
        // recopied the whole datagram as the payload went in, and `header.marshal()` allocated a
        // separate buffer for 12 bytes purely to copy them here -- two allocations per datagram,
        // ~180 datagrams a second on the main stream alone.
        let mut writer = BytesWriter::with_capacity(self.marshalled_len());

        self.header.marshal_into(&mut writer)?;

        if self.header.extension_flag == 1 {
            writer.write_u16::<BigEndian>(self.header_extension_profile)?;
            writer.write_u16::<BigEndian>(self.header_extension_length)?;
            writer.write(&self.header_extension_payload[..])?;
        }

        writer.write(&self.payload[..])?;
        if self.header.padding_flag == 1 {
            writer.write(&self.padding[..])?;
        }

        Ok(writer.extract_current_bytes())
    }
}

impl RtpPacket {
    pub fn new(header: RtpHeader) -> Self {
        Self {
            header,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::bytes_reader::BytesReader;
    use bytes::BytesMut;

    fn make_basic_rtp_bytes() -> BytesMut {
        // Minimal RTP packet: V=2, P=0, X=0, CC=0, M=0, PT=96, Seq=1, TS=1000, SSRC=12345
        // Then 4 bytes of payload
        let mut buf = BytesMut::new();
        // Byte 0: V=2(10), P=0, X=0, CC=0 => 0b10_0_0_0000 = 0x80
        buf.extend_from_slice(&[0x80]);
        // Byte 1: M=0, PT=96 => 0b0_1100000 = 0x60
        buf.extend_from_slice(&[0x60]);
        // Bytes 2-3: Sequence number = 1
        buf.extend_from_slice(&[0x00, 0x01]);
        // Bytes 4-7: Timestamp = 1000 = 0x000003E8
        buf.extend_from_slice(&[0x00, 0x00, 0x03, 0xE8]);
        // Bytes 8-11: SSRC = 12345 = 0x00003039
        buf.extend_from_slice(&[0x00, 0x00, 0x30, 0x39]);
        // Payload
        buf.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        buf
    }

    #[test]
    fn test_rtp_packet_new() {
        let header = RtpHeader::default();
        let pkt = RtpPacket::new(header);
        assert!(pkt.payload.is_empty());
        assert!(pkt.padding.is_empty());
        assert_eq!(pkt.header_extension_profile, 0);
        assert_eq!(pkt.header_extension_length, 0);
    }

    #[test]
    fn test_rtp_packet_default() {
        let pkt = RtpPacket::default();
        assert!(pkt.payload.is_empty());
        assert_eq!(pkt.header.version, 2);
    }

    #[test]
    fn test_rtp_packet_unmarshal_basic() {
        let data = make_basic_rtp_bytes();
        let mut reader = BytesReader::new(data);
        let pkt = RtpPacket::unmarshal(&mut reader).unwrap();

        assert_eq!(pkt.header.version, 2);
        assert_eq!(pkt.header.padding_flag, 0);
        assert_eq!(pkt.header.extension_flag, 0);
        assert_eq!(pkt.header.cc, 0);
        assert_eq!(pkt.header.payload_type, 96);
        assert_eq!(pkt.header.seq_number, 1);
        assert_eq!(pkt.header.timestamp, 1000);
        assert_eq!(pkt.header.ssrc, 12345);
        assert_eq!(pkt.payload.as_ref(), &[0xAA, 0xBB, 0xCC, 0xDD]);
        assert!(pkt.padding.is_empty());
    }

    #[test]
    fn test_rtp_packet_marshal_roundtrip() {
        let data = make_basic_rtp_bytes();
        let mut reader = BytesReader::new(data.clone());
        let pkt = RtpPacket::unmarshal(&mut reader).unwrap();

        let marshaled = pkt.marshal().unwrap();
        assert_eq!(&marshaled[..], &data[..]);
    }

    #[test]
    fn test_rtp_packet_with_extension() {
        let mut buf = BytesMut::new();
        // V=2, P=0, X=1, CC=0 => 0b10_0_1_0000 = 0x90
        buf.extend_from_slice(&[0x90]);
        // M=0, PT=96
        buf.extend_from_slice(&[0x60]);
        // Seq=2
        buf.extend_from_slice(&[0x00, 0x02]);
        // Timestamp=2000
        buf.extend_from_slice(&[0x00, 0x00, 0x07, 0xD0]);
        // SSRC=100
        buf.extend_from_slice(&[0x00, 0x00, 0x00, 0x64]);
        // Extension profile = 0xBEDE (RTP one-byte header extension)
        buf.extend_from_slice(&[0xBE, 0xDE]);
        // Extension length = 1 (in 32-bit words, so 4 bytes)
        buf.extend_from_slice(&[0x00, 0x01]);
        // Extension payload (4 bytes)
        buf.extend_from_slice(&[0x10, 0x01, 0x00, 0x00]);
        // Payload
        buf.extend_from_slice(&[0xFF, 0xEE]);

        let mut reader = BytesReader::new(buf.clone());
        let pkt = RtpPacket::unmarshal(&mut reader).unwrap();

        assert_eq!(pkt.header.extension_flag, 1);
        assert_eq!(pkt.header_extension_profile, 0xBEDE);
        assert_eq!(pkt.header_extension_length, 1);
        assert_eq!(pkt.header_extension_payload.len(), 4);
        assert_eq!(pkt.payload.as_ref(), &[0xFF, 0xEE]);

        // Roundtrip
        let marshaled = pkt.marshal().unwrap();
        assert_eq!(&marshaled[..], &buf[..]);
    }

    #[test]
    fn test_rtp_packet_with_padding() {
        let mut buf = BytesMut::new();
        // V=2, P=1, X=0, CC=0 => 0b10_1_0_0000 = 0xA0
        buf.extend_from_slice(&[0xA0]);
        // M=0, PT=96
        buf.extend_from_slice(&[0x60]);
        // Seq=3
        buf.extend_from_slice(&[0x00, 0x03]);
        // Timestamp=3000
        buf.extend_from_slice(&[0x00, 0x00, 0x0B, 0xB8]);
        // SSRC=200
        buf.extend_from_slice(&[0x00, 0x00, 0x00, 0xC8]);
        // Payload (2 bytes) + Padding (3 bytes, last byte = padding count)
        buf.extend_from_slice(&[0xAA, 0xBB, 0x00, 0x00, 0x03]);

        let mut reader = BytesReader::new(buf);
        let pkt = RtpPacket::unmarshal(&mut reader).unwrap();

        assert_eq!(pkt.header.padding_flag, 1);
        assert_eq!(pkt.payload.as_ref(), &[0xAA, 0xBB]);
        assert_eq!(pkt.padding.as_ref(), &[0x00, 0x00, 0x03]);
    }

    #[test]
    fn test_rtp_packet_marshal_with_padding() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0xA0, 0x60, 0x00, 0x03]);
        buf.extend_from_slice(&[0x00, 0x00, 0x0B, 0xB8]);
        buf.extend_from_slice(&[0x00, 0x00, 0x00, 0xC8]);
        buf.extend_from_slice(&[0xAA, 0xBB, 0x00, 0x00, 0x03]);

        let mut reader = BytesReader::new(buf.clone());
        let pkt = RtpPacket::unmarshal(&mut reader).unwrap();
        let marshaled = pkt.marshal().unwrap();
        assert_eq!(&marshaled[..], &buf[..]);
    }

    #[test]
    fn test_rtp_packet_unmarshal_too_short() {
        // Only 4 bytes - not enough for a valid RTP header
        let buf = BytesMut::from(&[0x80, 0x60, 0x00, 0x01][..]);
        let mut reader = BytesReader::new(buf);
        let result = RtpPacket::unmarshal(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_rtp_packet_unmarshal_empty() {
        let buf = BytesMut::new();
        let mut reader = BytesReader::new(buf);
        let result = RtpPacket::unmarshal(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_rtp_packet_clone() {
        let data = make_basic_rtp_bytes();
        let mut reader = BytesReader::new(data);
        let pkt = RtpPacket::unmarshal(&mut reader).unwrap();
        let cloned = pkt.clone();

        assert_eq!(pkt.header.seq_number, cloned.header.seq_number);
        assert_eq!(pkt.header.timestamp, cloned.header.timestamp);
        assert_eq!(pkt.payload, cloned.payload);
    }

    #[test]
    fn test_rtp_packet_debug() {
        let pkt = RtpPacket::default();
        let debug = format!("{:?}", pkt);
        assert!(debug.contains("RtpPacket"));
    }

    #[test]
    fn test_rtp_packet_with_extension_and_padding() {
        let mut buf = BytesMut::new();
        // V=2, P=1, X=1, CC=0 => 0b10_1_1_0000 = 0xB0
        buf.extend_from_slice(&[0xB0]);
        // M=0, PT=96
        buf.extend_from_slice(&[0x60]);
        // Seq=5
        buf.extend_from_slice(&[0x00, 0x05]);
        // Timestamp=5000
        buf.extend_from_slice(&[0x00, 0x00, 0x13, 0x88]);
        // SSRC=300
        buf.extend_from_slice(&[0x00, 0x00, 0x01, 0x2C]);
        // Extension profile = 0xABCD
        buf.extend_from_slice(&[0xAB, 0xCD]);
        // Extension length = 1 (4 bytes)
        buf.extend_from_slice(&[0x00, 0x01]);
        // Extension payload
        buf.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]);
        // Payload (2 bytes) + padding (2 bytes, last byte = 2)
        buf.extend_from_slice(&[0xDD, 0xEE, 0x00, 0x02]);

        let mut reader = BytesReader::new(buf.clone());
        let pkt = RtpPacket::unmarshal(&mut reader).unwrap();

        assert_eq!(pkt.header.extension_flag, 1);
        assert_eq!(pkt.header.padding_flag, 1);
        assert_eq!(pkt.header_extension_profile, 0xABCD);
        assert_eq!(pkt.payload.as_ref(), &[0xDD, 0xEE]);
        assert_eq!(pkt.padding.as_ref(), &[0x00, 0x02]);

        // Roundtrip
        let marshaled = pkt.marshal().unwrap();
        assert_eq!(&marshaled[..], &buf[..]);
    }

    #[test]
    fn test_rtp_packet_marshal_no_extension_no_padding() {
        let header = RtpHeader {
            version: 2,
            padding_flag: 0,
            extension_flag: 0,
            cc: 0,
            marker: 0,
            payload_type: 111,
            seq_number: 42,
            timestamp: 9999,
            ssrc: 54321,
            ..Default::default()
        };

        let mut pkt = RtpPacket::new(header);
        pkt.payload = BytesMut::from(&[0x01, 0x02, 0x03][..]);

        let marshaled = pkt.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let pkt2 = RtpPacket::unmarshal(&mut reader).unwrap();

        assert_eq!(pkt2.header.payload_type, 111);
        assert_eq!(pkt2.header.seq_number, 42);
        assert_eq!(pkt2.header.timestamp, 9999);
        assert_eq!(pkt2.header.ssrc, 54321);
        assert_eq!(pkt2.payload.as_ref(), &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_rtp_packet_empty_payload() {
        let mut buf = BytesMut::new();
        // V=2, P=0, X=0, CC=0
        buf.extend_from_slice(&[0x80]);
        // M=0, PT=96
        buf.extend_from_slice(&[0x60]);
        // Seq=0
        buf.extend_from_slice(&[0x00, 0x00]);
        // Timestamp=0
        buf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        // SSRC=0
        buf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        // No payload

        let mut reader = BytesReader::new(buf);
        let pkt = RtpPacket::unmarshal(&mut reader).unwrap();
        assert!(pkt.payload.is_empty());
    }

    /// `marshalled_len` must equal what `marshal` actually writes.
    ///
    /// This is asserted rather than assumed because getting it wrong is *silent*: too small only
    /// costs the reallocation the pre-sizing existed to avoid, and too large only wastes memory.
    /// Neither shows up as a test failure anywhere else, so the sizing would quietly rot.
    #[test]
    fn marshalled_len_matches_marshal_output_for_every_shape() {
        let mut reader = BytesReader::new(make_basic_rtp_bytes());
        let base = RtpPacket::unmarshal(&mut reader).unwrap();

        // Plain packet: fixed header + payload.
        assert_eq!(
            base.marshalled_len(),
            base.marshal().unwrap().len(),
            "plain"
        );

        // With CSRCs, which extend the header by a word each.
        let mut with_csrcs = base.clone();
        with_csrcs.header.csrcs = vec![1, 2, 3];
        assert_eq!(
            with_csrcs.marshalled_len(),
            with_csrcs.marshal().unwrap().len(),
            "csrcs"
        );

        // With a header extension: 2-byte profile + 2-byte length + payload.
        let mut with_ext = base.clone();
        with_ext.header.extension_flag = 1;
        with_ext.header_extension_profile = 0xBEDE;
        with_ext.header_extension_length = 2;
        with_ext.header_extension_payload = BytesMut::from(&[1u8, 2, 3, 4, 5, 6, 7, 8][..]);
        assert_eq!(
            with_ext.marshalled_len(),
            with_ext.marshal().unwrap().len(),
            "extension"
        );

        // With padding.
        let mut with_padding = base.clone();
        with_padding.header.padding_flag = 1;
        with_padding.padding = BytesMut::from(&[0u8, 0, 0, 4][..]);
        assert_eq!(
            with_padding.marshalled_len(),
            with_padding.marshal().unwrap().len(),
            "padding"
        );
    }
}
