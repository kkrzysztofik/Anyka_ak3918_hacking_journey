use {
    super::errors::FlvMuxerError, crate::io::bytes_writer::BytesWriter, byteorder::BigEndian,
    bytes::BytesMut,
};

const FLV_HEADER_AV: [u8; 9] = [
    0x46, // 'F'
    0x4c, //'L'
    0x56, //'V'
    0x01, //version
    0x05, // flags: 0x04 audio + 0x01 video
    0x00, 0x00, 0x00, 0x09, //flv header size
]; // 9
const FLV_HEADER_JUST_AUDIO: [u8; 9] = [
    0x46, // 'F'
    0x4c, //'L'
    0x56, //'V'
    0x01, //version
    0x04, // flags: 0x04 audio only
    0x00, 0x00, 0x00, 0x09, //flv header size
]; // 9
const FLV_HEADER_JUST_VIDEO: [u8; 9] = [
    0x46, // 'F'
    0x4c, //'L'
    0x56, //'V'
    0x01, //version
    0x01, // flags: 0x01 video only
    0x00, 0x00, 0x00, 0x09, //flv header size
]; // 9
const FLV_HEADER_EMPTY_AV: [u8; 9] = [
    0x46, // 'F'
    0x4c, //'L'
    0x56, //'V'
    0x01, //version
    0x00, // flags: no audio/video
    0x00, 0x00, 0x00, 0x09, //flv header size
]; // 9
pub const HEADER_LENGTH: u32 = 11;
pub struct FlvMuxer {
    pub writer: BytesWriter,
}

impl Default for FlvMuxer {
    fn default() -> Self {
        Self::new()
    }
}

impl FlvMuxer {
    pub fn new() -> Self {
        Self {
            writer: BytesWriter::new(),
        }
    }

    pub fn write_flv_header(
        &mut self,
        has_audio: bool,
        has_video: bool,
    ) -> Result<(), FlvMuxerError> {
        if has_audio && has_video {
            self.writer.write(&FLV_HEADER_AV)?;
        } else if has_audio {
            self.writer.write(&FLV_HEADER_JUST_AUDIO)?;
        } else if has_video {
            self.writer.write(&FLV_HEADER_JUST_VIDEO)?;
        } else {
            self.writer.write(&FLV_HEADER_EMPTY_AV)?;
        }
        Ok(())
    }

    pub fn write_flv_tag_header(
        &mut self,
        tag_type: u8,
        data_size: u32,
        timestamp: u32,
    ) -> Result<(), FlvMuxerError> {
        //tag type
        self.writer.write_u8(tag_type)?;
        //data size
        self.writer.write_u24::<BigEndian>(data_size)?;
        //timestamp
        self.writer.write_u24::<BigEndian>(timestamp & 0xffffff)?;
        //timestamp extended.
        let timestamp_ext = (timestamp >> 24 & 0xff) as u8;
        self.writer.write_u8(timestamp_ext)?;
        //stream id
        self.writer.write_u24::<BigEndian>(0)?;

        Ok(())
    }

    pub fn write_flv_tag_body(&mut self, body: BytesMut) -> Result<(), FlvMuxerError> {
        self.writer.write(&body[..])?;
        Ok(())
    }

    pub fn write_previous_tag_size(&mut self, size: u32) -> Result<(), FlvMuxerError> {
        self.writer.write_u32::<BigEndian>(size)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    // ============================================
    // Construction Tests
    // ============================================

    #[test]
    fn test_flv_muxer_new() {
        let muxer = FlvMuxer::new();
        assert_eq!(muxer.writer.len(), 0);
    }

    #[test]
    fn test_flv_muxer_default() {
        let muxer = FlvMuxer::default();
        assert_eq!(muxer.writer.len(), 0);
    }

    // ============================================
    // FLV Header Generation Tests
    // ============================================

    #[test]
    fn test_write_flv_header_audio_video() {
        let mut muxer = FlvMuxer::new();
        muxer.write_flv_header(true, true).unwrap();

        let bytes = muxer.writer.get_current_bytes();
        assert_eq!(bytes.len(), 9);
        assert_eq!(&bytes[0..3], b"FLV");
        assert_eq!(bytes[3], 0x01); // version
        assert_eq!(bytes[4], 0x05); // audio + video flags
        assert_eq!(&bytes[5..9], &[0x00, 0x00, 0x00, 0x09]); // header size
    }

    #[test]
    fn test_write_flv_header_audio_only() {
        let mut muxer = FlvMuxer::new();
        muxer.write_flv_header(true, false).unwrap();

        let bytes = muxer.writer.get_current_bytes();
        assert_eq!(bytes.len(), 9);
        assert_eq!(&bytes[0..3], b"FLV");
        assert_eq!(bytes[4], 0x04); // audio only
    }

    #[test]
    fn test_write_flv_header_video_only() {
        let mut muxer = FlvMuxer::new();
        muxer.write_flv_header(false, true).unwrap();

        let bytes = muxer.writer.get_current_bytes();
        assert_eq!(bytes.len(), 9);
        assert_eq!(&bytes[0..3], b"FLV");
        assert_eq!(bytes[4], 0x01); // video only
    }

    #[test]
    fn test_write_flv_header_empty() {
        let mut muxer = FlvMuxer::new();
        muxer.write_flv_header(false, false).unwrap();

        let bytes = muxer.writer.get_current_bytes();
        assert_eq!(bytes.len(), 9);
        assert_eq!(&bytes[0..3], b"FLV");
        assert_eq!(bytes[4], 0x00); // no audio, no video
    }

    // ============================================
    // Tag Header Tests
    // ============================================

    #[test]
    fn test_write_flv_tag_header_audio() {
        let mut muxer = FlvMuxer::new();
        muxer.write_flv_tag_header(8, 100, 0).unwrap(); // Audio tag type = 8

        let bytes = muxer.writer.get_current_bytes();
        assert_eq!(bytes.len(), 11);
        assert_eq!(bytes[0], 8); // tag type
        assert_eq!(&bytes[1..4], &[0x00, 0x00, 0x64]); // data size = 100
        assert_eq!(&bytes[4..7], &[0x00, 0x00, 0x00]); // timestamp = 0
        assert_eq!(bytes[7], 0x00); // timestamp extended = 0
        assert_eq!(&bytes[8..11], &[0x00, 0x00, 0x00]); // stream id = 0
    }

    #[test]
    fn test_write_flv_tag_header_video() {
        let mut muxer = FlvMuxer::new();
        muxer.write_flv_tag_header(9, 200, 1000).unwrap(); // Video tag type = 9

        let bytes = muxer.writer.get_current_bytes();
        assert_eq!(bytes.len(), 11);
        assert_eq!(bytes[0], 9); // tag type
        assert_eq!(&bytes[1..4], &[0x00, 0x00, 0xC8]); // data size = 200
        assert_eq!(&bytes[4..7], &[0x00, 0x03, 0xE8]); // timestamp = 1000
        assert_eq!(bytes[7], 0x00); // timestamp extended = 0
    }

    #[test]
    fn test_write_flv_tag_header_extended_timestamp() {
        let mut muxer = FlvMuxer::new();
        // Timestamp > 24 bits (0x01000000 = 16777216)
        let timestamp = 0x01000000;
        muxer.write_flv_tag_header(9, 100, timestamp).unwrap();

        let bytes = muxer.writer.get_current_bytes();
        assert_eq!(bytes.len(), 11);
        // Lower 24 bits should be 0
        assert_eq!(&bytes[4..7], &[0x00, 0x00, 0x00]);
        // Extended timestamp should be 0x01
        assert_eq!(bytes[7], 0x01);
    }

    #[test]
    fn test_write_flv_tag_header_large_timestamp() {
        let mut muxer = FlvMuxer::new();
        // Maximum 32-bit timestamp
        let timestamp = 0xFFFFFFFF;
        muxer.write_flv_tag_header(9, 100, timestamp).unwrap();

        let bytes = muxer.writer.get_current_bytes();
        // Lower 24 bits should be 0xFFFFFF
        assert_eq!(&bytes[4..7], &[0xFF, 0xFF, 0xFF]);
        // Extended timestamp should be 0xFF
        assert_eq!(bytes[7], 0xFF);
    }

    #[test]
    fn test_write_flv_tag_header_large_data_size() {
        let mut muxer = FlvMuxer::new();
        // Data size = 0xFFFFFF (max 24-bit value)
        muxer.write_flv_tag_header(9, 0xFFFFFF, 0).unwrap();

        let bytes = muxer.writer.get_current_bytes();
        assert_eq!(&bytes[1..4], &[0xFF, 0xFF, 0xFF]);
    }

    // ============================================
    // Tag Body Tests
    // ============================================

    #[test]
    fn test_write_flv_tag_body() {
        let mut muxer = FlvMuxer::new();
        let mut body = BytesMut::new();
        body.extend_from_slice(&[1, 2, 3, 4, 5]);
        muxer.write_flv_tag_body(body).unwrap();

        let bytes = muxer.writer.get_current_bytes();
        assert_eq!(bytes.len(), 5);
        assert_eq!(&bytes[..], &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_write_flv_tag_body_empty() {
        let mut muxer = FlvMuxer::new();
        let body = BytesMut::new();
        muxer.write_flv_tag_body(body).unwrap();

        let bytes = muxer.writer.get_current_bytes();
        assert!(bytes.is_empty());
    }

    #[test]
    fn test_write_flv_tag_body_large() {
        let mut muxer = FlvMuxer::new();
        let mut body = BytesMut::new();
        body.extend_from_slice(&vec![0xAA; 1000]);
        muxer.write_flv_tag_body(body).unwrap();

        let bytes = muxer.writer.get_current_bytes();
        assert_eq!(bytes.len(), 1000);
    }

    // ============================================
    // Previous Tag Size Tests
    // ============================================

    #[test]
    fn test_write_previous_tag_size() {
        let mut muxer = FlvMuxer::new();
        muxer.write_previous_tag_size(100).unwrap();

        let bytes = muxer.writer.get_current_bytes();
        assert_eq!(bytes.len(), 4);
        assert_eq!(&bytes[..], &[0x00, 0x00, 0x00, 0x64]); // 100 in big endian
    }

    #[test]
    fn test_write_previous_tag_size_zero() {
        let mut muxer = FlvMuxer::new();
        muxer.write_previous_tag_size(0).unwrap();

        let bytes = muxer.writer.get_current_bytes();
        assert_eq!(&bytes[..], &[0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_write_previous_tag_size_max() {
        let mut muxer = FlvMuxer::new();
        muxer.write_previous_tag_size(0xFFFFFFFF).unwrap();

        let bytes = muxer.writer.get_current_bytes();
        assert_eq!(&bytes[..], &[0xFF, 0xFF, 0xFF, 0xFF]);
    }

    // ============================================
    // Complete FLV Tag Tests
    // ============================================

    #[test]
    fn test_complete_flv_tag_audio() {
        let mut muxer = FlvMuxer::new();

        // Write tag header
        let mut body = BytesMut::new();
        body.extend_from_slice(&[0xAF, 0x00, 0x10, 0x00]); // AAC audio data
        let body_size = body.len() as u32;

        muxer.write_flv_tag_header(8, body_size, 0).unwrap();
        muxer.write_flv_tag_body(body).unwrap();
        muxer
            .write_previous_tag_size(HEADER_LENGTH + body_size)
            .unwrap();

        let bytes = muxer.writer.get_current_bytes();
        // Header (11) + Body (4) + Previous tag size (4) = 19
        assert_eq!(bytes.len(), 19);
        assert_eq!(bytes[0], 8); // Audio tag type
    }

    #[test]
    fn test_complete_flv_tag_video() {
        let mut muxer = FlvMuxer::new();

        // Write tag header
        let mut body = BytesMut::new();
        body.extend_from_slice(&[0x17, 0x00, 0x00, 0x00, 0x00]); // AVC video data
        let body_size = body.len() as u32;

        muxer.write_flv_tag_header(9, body_size, 1000).unwrap();
        muxer.write_flv_tag_body(body).unwrap();
        muxer
            .write_previous_tag_size(HEADER_LENGTH + body_size)
            .unwrap();

        let bytes = muxer.writer.get_current_bytes();
        // Header (11) + Body (5) + Previous tag size (4) = 20
        assert_eq!(bytes.len(), 20);
        assert_eq!(bytes[0], 9); // Video tag type
    }

    #[test]
    fn test_complete_flv_tag_sequence() {
        let mut muxer = FlvMuxer::new();

        // Write FLV header
        muxer.write_flv_header(true, true).unwrap();
        muxer.write_previous_tag_size(0).unwrap(); // First tag size is 0

        // First audio tag
        let mut body1 = BytesMut::new();
        body1.extend_from_slice(&[0xAF, 0x00]);
        let body_size1 = body1.len() as u32;
        muxer.write_flv_tag_header(8, body_size1, 0).unwrap();
        muxer.write_flv_tag_body(body1).unwrap();
        muxer
            .write_previous_tag_size(HEADER_LENGTH + body_size1)
            .unwrap();

        // Second video tag
        let mut body2 = BytesMut::new();
        body2.extend_from_slice(&[0x17, 0x00]);
        let body_size2 = body2.len() as u32;
        muxer.write_flv_tag_header(9, body_size2, 33).unwrap();
        muxer.write_flv_tag_body(body2).unwrap();
        muxer
            .write_previous_tag_size(HEADER_LENGTH + body_size2)
            .unwrap();

        let bytes = muxer.writer.get_current_bytes();
        // FLV header (9) + PreviousTagSize0 (4) + Tag1 (11+2+4) + Tag2 (11+2+4) = 47
        assert_eq!(bytes.len(), 47);
    }

    // ============================================
    // Timestamp Handling Tests
    // ============================================

    #[test]
    fn test_timestamp_24_bit_range() {
        let mut muxer = FlvMuxer::new();
        // Maximum 24-bit timestamp
        let timestamp = 0x00FFFFFF;
        muxer.write_flv_tag_header(9, 100, timestamp).unwrap();

        let bytes = muxer.writer.get_current_bytes();
        assert_eq!(&bytes[4..7], &[0xFF, 0xFF, 0xFF]);
        assert_eq!(bytes[7], 0x00); // Extended should be 0
    }

    #[test]
    fn test_timestamp_32_bit_range() {
        let mut muxer = FlvMuxer::new();
        // Timestamp requiring extended byte
        let timestamp = 0x01000000;
        muxer.write_flv_tag_header(9, 100, timestamp).unwrap();

        let bytes = muxer.writer.get_current_bytes();
        assert_eq!(&bytes[4..7], &[0x00, 0x00, 0x00]); // Lower 24 bits
        assert_eq!(bytes[7], 0x01); // Extended byte
    }

    #[test]
    fn test_timestamp_various_values() {
        let test_timestamps = vec![0, 1, 1000, 0x00FFFFFF, 0x01000000, 0xFFFFFFFF];

        for timestamp in test_timestamps {
            let mut muxer = FlvMuxer::new();
            muxer.write_flv_tag_header(9, 100, timestamp).unwrap();

            let bytes = muxer.writer.get_current_bytes();
            let lower_24 = u32::from_be_bytes([0, bytes[4], bytes[5], bytes[6]]);
            let extended = bytes[7] as u32;
            let reconstructed = lower_24 | (extended << 24);

            assert_eq!(reconstructed, timestamp);
        }
    }

    // ============================================
    // Property-based Tests (proptest)
    // ============================================

    use proptest::prelude::*;
    proptest! {
        #[test]
        fn test_flv_tag_header_property_roundtrip(tag_type in 8u8..=18u8, data_size in 0u32..=0xFFFFFFu32, timestamp in 0u32..=u32::MAX) {
            let mut muxer = FlvMuxer::new();
            muxer.write_flv_tag_header(tag_type, data_size, timestamp).unwrap();

            let bytes = muxer.writer.get_current_bytes();
            assert_eq!(bytes.len(), 11);
            assert_eq!(bytes[0], tag_type);

            // Verify data size
            let read_data_size = u32::from_be_bytes([0, bytes[1], bytes[2], bytes[3]]);
            assert_eq!(read_data_size, data_size);

            // Verify timestamp reconstruction
            let lower_24 = u32::from_be_bytes([0, bytes[4], bytes[5], bytes[6]]);
            let extended = bytes[7] as u32;
            let reconstructed = lower_24 | (extended << 24);
            assert_eq!(reconstructed, timestamp);
        }
    }
}
