use crate::bytesio::bytes_writer::BytesWriter;

use {
    super::{Marshal, Unmarshal},
    super::{
        define,
        errors::{FlvDemuxerError, FlvMuxerError},
    },
    crate::bytesio::bytes_reader::BytesReader,
    bytes::BytesMut,
};

#[derive(Clone, Debug, Default)]
pub struct AudioTagHeader {
    //1010 11 1 1
    /*
        SoundFormat: UB[4]
        0 = Linear PCM, platform endian
        1 = ADPCM
        2 = MP3
        3 = Linear PCM, little endian
        4 = Nellymoser 16-kHz mono
        5 = Nellymoser 8-kHz mono
        6 = Nellymoser
        7 = G.711 A-law logarithmic PCM
        8 = G.711 mu-law logarithmic PCM
        9 = reserved
        10 = AAC
        11 = Speex
        14 = MP3 8-Khz
        15 = Device-specific sound
        Formats 7, 8, 14, and 15 are reserved for internal use
        AAC is supported in Flash Player 9,0,115,0 and higher.
        Speex is supported in Flash Player 10 and higher.
    */
    pub sound_format: u8,
    /*
        SoundRate: UB[2]
        Sampling rate
        0 = 5.5-kHz For AAC: always 3
        1 = 11-kHz
        2 = 22-kHz
        3 = 44-kHz
    */
    pub sound_rate: u8,
    /*
        SoundSize: UB[1]
        0 = snd8Bit
        1 = snd16Bit
        Size of each sample.
        This parameter only pertains to uncompressed formats.
        Compressed formats always decode to 16 bits internally
    */
    pub sound_size: u8,
    /*
        SoundType: UB[1]
        0 = sndMono
        1 = sndStereo
        Mono or stereo sound For Nellymoser: always 0
        For AAC: always 1
    */
    pub sound_type: u8,

    /*
        0: AAC sequence header
        1: AAC raw
    */
    pub aac_packet_type: u8,
}

impl Unmarshal<&mut BytesReader, Result<Self, FlvDemuxerError>> for AudioTagHeader {
    fn unmarshal(reader: &mut BytesReader) -> Result<Self, FlvDemuxerError>
    where
        Self: Sized,
    {
        let mut tag_header = AudioTagHeader::default();

        let flags = reader.read_u8()?;
        tag_header.sound_format = flags >> 4;
        tag_header.sound_rate = (flags >> 2) & 0x03;
        tag_header.sound_size = (flags >> 1) & 0x01;
        tag_header.sound_type = flags & 0x01;

        if tag_header.sound_format == define::SoundFormat::AAC as u8 {
            tag_header.aac_packet_type = reader.read_u8()?;
        }

        Ok(tag_header)
    }
}

impl Marshal<Result<BytesMut, FlvMuxerError>> for AudioTagHeader {
    fn marshal(&self) -> Result<BytesMut, FlvMuxerError> {
        let mut writer = BytesWriter::default();

        let byte_1st =
            self.sound_format << 4 | self.sound_rate << 2 | self.sound_size << 1 | self.sound_type;
        writer.write_u8(byte_1st)?;

        if self.sound_format == define::SoundFormat::AAC as u8 {
            writer.write_u8(self.aac_packet_type)?;
        }

        Ok(writer.extract_current_bytes())
    }
}

#[derive(Clone, Debug, Default)]
pub struct VideoTagHeader {
    /*
        1: keyframe (for AVC, a seekable frame)
        2: inter frame (for AVC, a non- seekable frame)
        3: disposable inter frame (H.263 only)
        4: generated keyframe (reserved for server use only)
        5: video info/command frame
    */
    pub frame_type: u8,
    /*
        1: JPEG (currently unused)
        2: Sorenson H.263
        3: Screen video
        4: On2 VP6
        5: On2 VP6 with alpha channel
        6: Screen video version 2
        7: AVC
        12: HEVC
    */
    pub codec_id: u8,
    /*
        0: AVC sequence header
        1: AVC NALU
        2: AVC end of sequence (lower level NALU sequence ender is not required or supported)
    */
    pub avc_packet_type: u8,
    pub composition_time: i32,
}

impl Unmarshal<&mut BytesReader, Result<Self, FlvDemuxerError>> for VideoTagHeader {
    fn unmarshal(reader: &mut BytesReader) -> Result<Self, FlvDemuxerError>
    where
        Self: Sized,
    {
        let mut tag_header = VideoTagHeader::default();

        let flags = reader.read_u8()?;
        tag_header.frame_type = flags >> 4;
        tag_header.codec_id = flags & 0x0f;

        if tag_header.codec_id == define::AvcCodecId::H264 as u8
            || tag_header.codec_id == define::AvcCodecId::HEVC as u8
        {
            tag_header.avc_packet_type = reader.read_u8()?;
            tag_header.composition_time = 0;

            //bigend 3bytes
            for _ in 0..3 {
                let time = reader.read_u8()?;
                //print!("==time0=={}\n", time);
                //print!("==time1=={}\n", self.tag.composition_time);
                tag_header.composition_time = (tag_header.composition_time << 8) + time as i32;
            }
            //transfer to signed i24
            if tag_header.composition_time & (1 << 23) != 0 {
                let sign_extend_mask = 0xff_ff << 23;
                // Sign extend the value
                tag_header.composition_time |= sign_extend_mask
            }
        }

        Ok(tag_header)
    }
}

impl Marshal<Result<BytesMut, FlvMuxerError>> for VideoTagHeader {
    fn marshal(&self) -> Result<BytesMut, FlvMuxerError> {
        let mut writer = BytesWriter::default();

        let byte_1st = self.frame_type << 4 | self.codec_id;
        writer.write_u8(byte_1st)?;

        if self.codec_id == define::AvcCodecId::H264 as u8
            || self.codec_id == define::AvcCodecId::HEVC as u8
        {
            writer.write_u8(self.avc_packet_type)?;

            let cts = self.composition_time as u32;
            writer.write_u8(((cts >> 16) & 0xFF) as u8)?;
            writer.write_u8(((cts >> 8) & 0xFF) as u8)?;
            writer.write_u8((cts & 0xFF) as u8)?;
        }

        Ok(writer.extract_current_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== AudioTagHeader Default Tests ==========

    #[test]
    fn test_audio_tag_header_default() {
        let header = AudioTagHeader::default();
        assert_eq!(header.sound_format, 0);
        assert_eq!(header.sound_rate, 0);
        assert_eq!(header.sound_size, 0);
        assert_eq!(header.sound_type, 0);
        assert_eq!(header.aac_packet_type, 0);
    }

    // ========== AudioTagHeader Marshal Tests ==========

    #[test]
    fn test_audio_tag_header_marshal_non_aac() {
        let header = AudioTagHeader {
            sound_format: 2, // MP3
            sound_rate: 3,   // 44kHz
            sound_size: 1,   // 16-bit
            sound_type: 1,   // Stereo
            aac_packet_type: 0,
        };

        let result = header.marshal().unwrap();
        // MP3 format: should only write 1 byte (no AAC packet type)
        assert_eq!(result.len(), 1);

        // Verify byte encoding: format(4) << 4 | rate(2) << 2 | size(1) << 1 | type(1)
        // 2 << 4 | 3 << 2 | 1 << 1 | 1 = 32 | 12 | 2 | 1 = 47 (0x2F)
        assert_eq!(result[0], 0x2F);
    }

    #[test]
    fn test_audio_tag_header_marshal_aac() {
        let header = AudioTagHeader {
            sound_format: 10, // AAC
            sound_rate: 3,    // 44kHz
            sound_size: 1,    // 16-bit
            sound_type: 1,    // Stereo
            aac_packet_type: 1,
        };

        let result = header.marshal().unwrap();
        // AAC format: should write 2 bytes (flags + AAC packet type)
        assert_eq!(result.len(), 2);

        // Verify byte encoding: format(10) << 4 | rate(3) << 2 | size(1) << 1 | type(1)
        // 10 << 4 | 3 << 2 | 1 << 1 | 1 = 160 | 12 | 2 | 1 = 175 (0xAF)
        assert_eq!(result[0], 0xAF);
        assert_eq!(result[1], 1); // AAC raw data
    }

    // ========== AudioTagHeader Unmarshal Tests ==========

    #[test]
    fn test_audio_tag_header_unmarshal_non_aac() {
        // MP3 format: format(2) << 4 | rate(3) << 2 | size(1) << 1 | type(1) = 0x2F
        let data = BytesMut::from(&[0x2F][..]);
        let mut reader = BytesReader::new(data);

        let header = AudioTagHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.sound_format, 2);
        assert_eq!(header.sound_rate, 3);
        assert_eq!(header.sound_size, 1);
        assert_eq!(header.sound_type, 1);
    }

    #[test]
    fn test_audio_tag_header_unmarshal_aac() {
        // AAC format: 0xAF (flags) + 0x01 (AAC raw)
        let data = BytesMut::from(&[0xAF, 0x01][..]);
        let mut reader = BytesReader::new(data);

        let header = AudioTagHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.sound_format, 10); // AAC
        assert_eq!(header.sound_rate, 3);
        assert_eq!(header.sound_size, 1);
        assert_eq!(header.sound_type, 1);
        assert_eq!(header.aac_packet_type, 1);
    }

    #[test]
    fn test_audio_tag_header_unmarshal_aac_seqhdr() {
        // AAC sequence header: 0xAF (flags) + 0x00 (sequence header)
        let data = BytesMut::from(&[0xAF, 0x00][..]);
        let mut reader = BytesReader::new(data);

        let header = AudioTagHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.sound_format, 10);
        assert_eq!(header.aac_packet_type, 0); // Sequence header
    }

    // ========== AudioTagHeader Round-Trip Tests ==========

    #[test]
    fn test_audio_tag_header_roundtrip_mp3() {
        let original = AudioTagHeader {
            sound_format: 2, // MP3
            sound_rate: 2,   // 22kHz
            sound_size: 1,   // 16-bit
            sound_type: 0,   // Mono
            aac_packet_type: 0,
        };

        let marshaled = original.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let decoded = AudioTagHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(decoded.sound_format, original.sound_format);
        assert_eq!(decoded.sound_rate, original.sound_rate);
        assert_eq!(decoded.sound_size, original.sound_size);
        assert_eq!(decoded.sound_type, original.sound_type);
    }

    #[test]
    fn test_audio_tag_header_roundtrip_aac() {
        let original = AudioTagHeader {
            sound_format: 10, // AAC
            sound_rate: 3,    // 44kHz
            sound_size: 1,    // 16-bit
            sound_type: 1,    // Stereo
            aac_packet_type: 1,
        };

        let marshaled = original.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let decoded = AudioTagHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(decoded.sound_format, original.sound_format);
        assert_eq!(decoded.sound_rate, original.sound_rate);
        assert_eq!(decoded.sound_size, original.sound_size);
        assert_eq!(decoded.sound_type, original.sound_type);
        assert_eq!(decoded.aac_packet_type, original.aac_packet_type);
    }

    // ========== VideoTagHeader Default Tests ==========

    #[test]
    fn test_video_tag_header_default() {
        let header = VideoTagHeader::default();
        assert_eq!(header.frame_type, 0);
        assert_eq!(header.codec_id, 0);
        assert_eq!(header.avc_packet_type, 0);
        assert_eq!(header.composition_time, 0);
    }

    // ========== VideoTagHeader Marshal Tests ==========

    #[test]
    fn test_video_tag_header_marshal_non_avc() {
        let header = VideoTagHeader {
            frame_type: 1, // Keyframe
            codec_id: 2,   // H.263
            avc_packet_type: 0,
            composition_time: 0,
        };

        let result = header.marshal().unwrap();
        // Non-AVC: should only write 1 byte
        assert_eq!(result.len(), 1);

        // frame_type(1) << 4 | codec_id(2) = 0x12
        assert_eq!(result[0], 0x12);
    }

    #[test]
    fn test_video_tag_header_marshal_h264_keyframe() {
        let header = VideoTagHeader {
            frame_type: 1,      // Keyframe
            codec_id: 7,        // AVC (H.264)
            avc_packet_type: 0, // Sequence header
            composition_time: 0,
        };

        let result = header.marshal().unwrap();
        // AVC: should write 5 bytes (flags + packet type + 3 bytes composition time)
        assert_eq!(result.len(), 5);

        // frame_type(1) << 4 | codec_id(7) = 0x17
        assert_eq!(result[0], 0x17);
        assert_eq!(result[1], 0); // Sequence header
    }

    #[test]
    fn test_video_tag_header_marshal_hevc() {
        let header = VideoTagHeader {
            frame_type: 1,      // Keyframe
            codec_id: 12,       // HEVC
            avc_packet_type: 1, // NALU
            composition_time: 100,
        };

        let result = header.marshal().unwrap();
        assert_eq!(result.len(), 5);

        // frame_type(1) << 4 | codec_id(12) = 0x1C
        assert_eq!(result[0], 0x1C);
        assert_eq!(result[1], 1); // NALU
    }

    // ========== VideoTagHeader Unmarshal Tests ==========

    #[test]
    fn test_video_tag_header_unmarshal_non_avc() {
        // H.263: frame_type(1) << 4 | codec_id(2) = 0x12
        let data = BytesMut::from(&[0x12][..]);
        let mut reader = BytesReader::new(data);

        let header = VideoTagHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.frame_type, 1);
        assert_eq!(header.codec_id, 2);
    }

    #[test]
    fn test_video_tag_header_unmarshal_h264() {
        // H.264 keyframe + sequence header + composition time 0
        let data = BytesMut::from(&[0x17, 0x00, 0x00, 0x00, 0x00][..]);
        let mut reader = BytesReader::new(data);

        let header = VideoTagHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.frame_type, 1);
        assert_eq!(header.codec_id, 7);
        assert_eq!(header.avc_packet_type, 0);
        assert_eq!(header.composition_time, 0);
    }

    #[test]
    fn test_video_tag_header_unmarshal_h264_with_cts() {
        // H.264 with composition time = 0x010203 (big-endian)
        let data = BytesMut::from(&[0x17, 0x01, 0x01, 0x02, 0x03][..]);
        let mut reader = BytesReader::new(data);

        let header = VideoTagHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.frame_type, 1);
        assert_eq!(header.codec_id, 7);
        assert_eq!(header.avc_packet_type, 1);
        // Big-endian: 0x01 << 16 | 0x02 << 8 | 0x03 = 66051
        assert_eq!(header.composition_time, 66051);
    }

    #[test]
    fn test_video_tag_header_unmarshal_negative_cts() {
        // H.264 with negative composition time (sign bit set in 24-bit value)
        // 0x800001 in big-endian: this has bit 23 set, so should be sign-extended
        let data = BytesMut::from(&[0x17, 0x01, 0x80, 0x00, 0x01][..]);
        let mut reader = BytesReader::new(data);

        let header = VideoTagHeader::unmarshal(&mut reader).unwrap();
        // The value 0x800001 with sign extension becomes negative
        assert!(header.composition_time < 0);
    }

    // ========== VideoTagHeader Round-Trip Tests ==========

    #[test]
    fn test_video_tag_header_roundtrip_h264_keyframe() {
        let original = VideoTagHeader {
            frame_type: 1,
            codec_id: 7, // H.264
            avc_packet_type: 0,
            composition_time: 0,
        };

        let marshaled = original.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let decoded = VideoTagHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(decoded.frame_type, original.frame_type);
        assert_eq!(decoded.codec_id, original.codec_id);
        assert_eq!(decoded.avc_packet_type, original.avc_packet_type);
        assert_eq!(decoded.composition_time, original.composition_time);
    }

    #[test]
    fn test_video_tag_header_roundtrip_h264_interframe() {
        let original = VideoTagHeader {
            frame_type: 2, // Inter frame
            codec_id: 7,   // H.264
            avc_packet_type: 1,
            composition_time: 33,
        };

        let marshaled = original.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let decoded = VideoTagHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(decoded.frame_type, original.frame_type);
        assert_eq!(decoded.codec_id, original.codec_id);
        assert_eq!(decoded.avc_packet_type, original.avc_packet_type);
        assert_eq!(decoded.composition_time, original.composition_time);
    }

    #[test]
    fn test_video_tag_header_roundtrip_hevc() {
        let original = VideoTagHeader {
            frame_type: 1,
            codec_id: 12, // HEVC
            avc_packet_type: 1,
            composition_time: 0,
        };

        let marshaled = original.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let decoded = VideoTagHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(decoded.frame_type, original.frame_type);
        assert_eq!(decoded.codec_id, original.codec_id);
        assert_eq!(decoded.avc_packet_type, original.avc_packet_type);
    }

    // ========== Clone and Debug Tests ==========

    #[test]
    fn test_audio_tag_header_clone() {
        let header = AudioTagHeader {
            sound_format: 10,
            sound_rate: 3,
            sound_size: 1,
            sound_type: 1,
            aac_packet_type: 1,
        };

        let cloned = header.clone();
        assert_eq!(cloned.sound_format, header.sound_format);
        assert_eq!(cloned.aac_packet_type, header.aac_packet_type);
    }

    #[test]
    fn test_audio_tag_header_debug() {
        let header = AudioTagHeader::default();
        let debug_str = format!("{:?}", header);
        assert!(debug_str.contains("AudioTagHeader"));
        assert!(debug_str.contains("sound_format"));
    }

    #[test]
    fn test_video_tag_header_clone() {
        let header = VideoTagHeader {
            frame_type: 1,
            codec_id: 7,
            avc_packet_type: 0,
            composition_time: 100,
        };

        let cloned = header.clone();
        assert_eq!(cloned.frame_type, header.frame_type);
        assert_eq!(cloned.composition_time, header.composition_time);
    }

    // ========== Unmarshal Error Path Tests ==========

    #[test]
    fn test_audio_tag_header_unmarshal_empty_reader() {
        let data = BytesMut::new();
        let mut reader = BytesReader::new(data);
        let result = AudioTagHeader::unmarshal(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_audio_tag_header_unmarshal_aac_truncated() {
        // AAC format byte but no second byte for aac_packet_type
        let data = BytesMut::from(&[0xAF][..]);
        let mut reader = BytesReader::new(data);
        let result = AudioTagHeader::unmarshal(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_video_tag_header_unmarshal_empty_reader() {
        let data = BytesMut::new();
        let mut reader = BytesReader::new(data);
        let result = VideoTagHeader::unmarshal(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_video_tag_header_unmarshal_h264_truncated() {
        // H.264 flags byte but no subsequent bytes for avc_packet_type/composition_time
        let data = BytesMut::from(&[0x17][..]);
        let mut reader = BytesReader::new(data);
        let result = VideoTagHeader::unmarshal(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_video_tag_header_unmarshal_hevc_truncated() {
        // HEVC flags byte but no subsequent bytes
        let data = BytesMut::from(&[0x1C][..]);
        let mut reader = BytesReader::new(data);
        let result = VideoTagHeader::unmarshal(&mut reader);
        assert!(result.is_err());
    }

    // ========== Additional Marshal Edge Cases ==========

    #[test]
    fn test_audio_tag_header_marshal_speex() {
        let header = AudioTagHeader {
            sound_format: 11, // Speex
            sound_rate: 0,
            sound_size: 1,
            sound_type: 0,
            aac_packet_type: 0,
        };
        let result = header.marshal().unwrap();
        assert_eq!(result.len(), 1); // Non-AAC: only 1 byte
        assert_eq!(result[0], 0xB2); // 11 << 4 | 0 << 2 | 1 << 1 | 0 = 176 + 2 = 178
    }

    #[test]
    fn test_video_tag_header_marshal_h264_with_large_cts() {
        let header = VideoTagHeader {
            frame_type: 2,             // Inter frame
            codec_id: 7,               // H.264
            avc_packet_type: 1,        // NALU
            composition_time: 8388607, // Max positive 23-bit value (0x7FFFFF)
        };
        let result = header.marshal().unwrap();
        assert_eq!(result.len(), 5);
        // CTS bytes: 0x7F, 0xFF, 0xFF
        assert_eq!(result[2], 0x7F);
        assert_eq!(result[3], 0xFF);
        assert_eq!(result[4], 0xFF);
    }

    #[test]
    fn test_video_tag_header_unmarshal_h264_zero_cts() {
        // H.264 inter frame, NALU, CTS = 0
        let data = BytesMut::from(&[0x27, 0x01, 0x00, 0x00, 0x00][..]);
        let mut reader = BytesReader::new(data);
        let header = VideoTagHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.frame_type, 2);
        assert_eq!(header.codec_id, 7);
        assert_eq!(header.avc_packet_type, 1);
        assert_eq!(header.composition_time, 0);
    }

    #[test]
    fn test_video_tag_header_unmarshal_hevc_keyframe() {
        // HEVC keyframe, sequence header
        let data = BytesMut::from(&[0x1C, 0x00, 0x00, 0x00, 0x00][..]);
        let mut reader = BytesReader::new(data);
        let header = VideoTagHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.frame_type, 1);
        assert_eq!(header.codec_id, 12);
        assert_eq!(header.avc_packet_type, 0);
        assert_eq!(header.composition_time, 0);
    }

    #[test]
    fn test_video_tag_header_roundtrip_non_avc() {
        let original = VideoTagHeader {
            frame_type: 2,
            codec_id: 2, // H.263
            avc_packet_type: 0,
            composition_time: 0,
        };
        let marshaled = original.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let decoded = VideoTagHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(decoded.frame_type, original.frame_type);
        assert_eq!(decoded.codec_id, original.codec_id);
    }

    // ========== Video Tag Header AVC End of Sequence ==========

    #[test]
    fn test_video_tag_header_avc_end_of_sequence() {
        let header = VideoTagHeader {
            frame_type: 1,
            codec_id: 7,
            avc_packet_type: 2, // End of sequence
            composition_time: 0,
        };
        let result = header.marshal().unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(result[1], 2); // avc_packet_type
    }
}
