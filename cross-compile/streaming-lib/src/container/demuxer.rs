use crate::container::{
    Unmarshal,
    flv_tag_header::{AudioTagHeader, VideoTagHeader},
};

use {
    super::{
        define::{AvcCodecId, FlvData, SoundFormat, aac_packet_type, avc_packet_type, tag_type},
        errors::FlvDemuxerError,
        mpeg4_aac::Mpeg4AacProcessor,
        mpeg4_avc::Mpeg4AvcProcessor,
    },
    crate::bytesio::bytes_reader::BytesReader,
    byteorder::BigEndian,
    bytes::BytesMut,
};

/*
 ** Flv Struct **
 +-------------------------------------------------------------------------------+
 | FLV header(9 bytes) | FLV body                                                |
 +-------------------------------------------------------------------------------+
 |                     | PreviousTagSize0(4 bytes)| Tag1|PreviousTagSize1|Tag2|...
 +-------------------------------------------------------------------------------+

 *** Flv Tag ***
 +-------------------------------------------------------------------------------------------------------------------------------+
 |                                                    Tag1                                                                       |
 +-------------------------------------------------------------------------------------------------------------------------------+
 |     Tag Header                                                                                                   |  Tag Data  |
 +-------------------------------------------------------------------------------------------------------------------------------+
 | Tag Type(1 byte) | Data Size(3 bytes) | Timestamp(3 bytes dts) | Timestamp Extended(1 byte) | Stream ID(3 bytes) |  Tag Data  |
 +-------------------------------------------------------------------------------------------------------------------------------+


  The Tag Data contains
  - video tag data
  - audio tag data

 **** Video Tag ****
 +-------------------------------------------------+
 |    Tag Data  (Video Tag)                        |
 +-------------------------------------------------+
 | FrameType(4 bits) | CodecID(4 bits) | Video Data|
 +-------------------------------------------------+

  The contents of Video Data depends on the codecID:
  2: H263VIDEOPACKET
  3: SCREENVIDEOPACKET
  4: VP6FLVVIDEOPACKET
  5: VP6FLVALPHAVIDEOPACKET
  6: SCREENV2VIDEOPACKET
  7: AVCVIDEOPACKE

 When the codecid equals 7, the Video Data's struct is as follows:

 +------------------------------------------------------------+
 |    Video Data  (codecID == 7)                              |
 +------------------------------------------------------------+
 | AVCPacketType(1 byte) | CompositionTime(3 bytes) | Payload |
 +------------------------------------------------------------+

 **** Audio Tag ****
 +----------------------------------------------------------------------------------------+
 |    Tag Data  (Audio Tag)                                                               |
 +----------------------------------------------------------------------------------------+
 | SoundFormat(4 bits) | SoundRate(2 bits) | SoundSize(1 bit) | SoundType(1 bit)| Payload |
 +----------------------------------------------------------------------------------------+

 reference: https://www.cnblogs.com/chyingp/p/flv-getting-started.html
*/

#[derive(Default)]
pub struct FlvDemuxerAudioData {
    pub has_data: bool,
    pub sound_format: u8,
    pub dts: i64,
    pub pts: i64,
    pub data: BytesMut,
}

impl FlvDemuxerAudioData {
    pub fn new() -> Self {
        Self {
            has_data: false,
            sound_format: 0,
            dts: 0,
            pts: 0,
            data: BytesMut::new(),
        }
    }
}
#[derive(Default)]
pub struct FlvDemuxerVideoData {
    pub frame_type: u8,
    pub codec_id: u8,
    pub dts: i64,
    pub pts: i64,
    pub data: BytesMut,
}

impl FlvDemuxerVideoData {
    pub fn new() -> Self {
        Self {
            codec_id: 0,
            dts: 0,
            pts: 0,
            frame_type: 0,
            data: BytesMut::new(),
        }
    }
}

#[derive(Default)]
pub struct FlvVideoTagDemuxer {
    avc_processor: Mpeg4AvcProcessor,
}

impl FlvVideoTagDemuxer {
    pub fn new() -> Self {
        Self {
            avc_processor: Mpeg4AvcProcessor::new(),
        }
    }
    pub fn demux(
        &mut self,
        timestamp: u32,
        data: BytesMut,
    ) -> Result<Option<FlvDemuxerVideoData>, FlvDemuxerError> {
        let mut reader = BytesReader::new(data);

        let tag_header = VideoTagHeader::unmarshal(&mut reader)?;
        if tag_header.codec_id == AvcCodecId::H264 as u8 {
            match tag_header.avc_packet_type {
                avc_packet_type::AVC_SEQHDR => {
                    self.avc_processor
                        .decoder_configuration_record_load(&mut reader)?;

                    return Ok(None);
                }
                avc_packet_type::AVC_NALU => {
                    let data = self.avc_processor.h264_mp4toannexb(&mut reader)?;

                    let video_data = FlvDemuxerVideoData {
                        codec_id: AvcCodecId::H264 as u8,
                        pts: timestamp as i64 + tag_header.composition_time as i64,
                        dts: timestamp as i64,
                        frame_type: tag_header.frame_type,
                        data,
                    };
                    //print!("flv demux video payload length {}\n", video_data.data.len());
                    return Ok(Some(video_data));
                }
                _ => {}
            }
        }

        Ok(None)
    }
}

#[derive(Default)]
pub struct FlvAudioTagDemuxer {
    aac_processor: Mpeg4AacProcessor,
}

impl FlvAudioTagDemuxer {
    pub fn new() -> Self {
        Self {
            aac_processor: Mpeg4AacProcessor::new(),
        }
    }

    pub fn demux(
        &mut self,
        timestamp: u32,
        data: BytesMut,
    ) -> Result<FlvDemuxerAudioData, FlvDemuxerError> {
        let mut reader = BytesReader::new(data);

        let tag_header = AudioTagHeader::unmarshal(&mut reader)?;
        self.aac_processor
            .extend_data(reader.extract_remaining_bytes());

        if tag_header.sound_format == SoundFormat::AAC as u8 {
            match tag_header.aac_packet_type {
                aac_packet_type::AAC_SEQHDR => {
                    if self.aac_processor.bytes_reader.len() >= 2 {
                        self.aac_processor.audio_specific_config_load()?;
                    }

                    return Ok(FlvDemuxerAudioData::new());
                }
                aac_packet_type::AAC_RAW => {
                    self.aac_processor.adts_save()?;

                    let audio_data = FlvDemuxerAudioData {
                        has_data: true,
                        sound_format: tag_header.sound_format,
                        pts: timestamp as i64,
                        dts: timestamp as i64,
                        data: self.aac_processor.bytes_writer.extract_current_bytes(),
                    };
                    //print!("flv demux audio payload length {}\n", audio_data.data.len());
                    return Ok(audio_data);
                }
                _ => {}
            }
        }

        Ok(FlvDemuxerAudioData::new())
    }
}

pub struct FlvDemuxer {
    bytes_reader: BytesReader,
}

impl FlvDemuxer {
    pub fn new(data: BytesMut) -> Self {
        Self {
            bytes_reader: BytesReader::new(data),
        }
    }

    pub fn read_flv_header(&mut self) -> Result<(), FlvDemuxerError> {
        /*flv header*/
        self.bytes_reader.read_bytes(9)?;
        Ok(())
    }

    pub fn read_flv_tag(&mut self) -> Result<Option<FlvData>, FlvDemuxerError> {
        /*previous_tag_size*/
        self.bytes_reader.read_u32::<BigEndian>()?;

        /*tag type*/
        let tag_type = self.bytes_reader.read_u8()?;
        /*data size*/
        let data_size = self.bytes_reader.read_u24::<BigEndian>()?;
        /*timestamp*/
        let timestamp = self.bytes_reader.read_u24::<BigEndian>()?;
        /*timestamp extended*/
        let timestamp_ext = self.bytes_reader.read_u8()?;
        /*stream id*/
        self.bytes_reader.read_u24::<BigEndian>()?;

        let dts: u32 = (timestamp & 0xffffff) | ((timestamp_ext as u32) << 24);

        /*data*/
        let body = self.bytes_reader.read_bytes(data_size as usize)?;

        match tag_type {
            tag_type::VIDEO => {
                return Ok(Some(FlvData::Video {
                    timestamp: dts,
                    data: body,
                }));
            }
            tag_type::AUDIO => {
                return Ok(Some(FlvData::Audio {
                    timestamp: dts,
                    data: body,
                }));
            }

            _ => {}
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::muxer::FlvMuxer;
    use bytes::BytesMut;

    // ============================================
    // FlvDemuxerAudioData Tests
    // ============================================

    #[test]
    fn test_flv_demuxer_audio_data_new() {
        let audio_data = FlvDemuxerAudioData::new();
        assert!(!audio_data.has_data);
        assert_eq!(audio_data.sound_format, 0);
        assert_eq!(audio_data.dts, 0);
        assert_eq!(audio_data.pts, 0);
        assert!(audio_data.data.is_empty());
    }

    #[test]
    fn test_flv_demuxer_audio_data_default() {
        let audio_data = FlvDemuxerAudioData::default();
        assert!(!audio_data.has_data);
        assert_eq!(audio_data.sound_format, 0);
    }

    // ============================================
    // FlvDemuxerVideoData Tests
    // ============================================

    #[test]
    fn test_flv_demuxer_video_data_new() {
        let video_data = FlvDemuxerVideoData::new();
        assert_eq!(video_data.frame_type, 0);
        assert_eq!(video_data.codec_id, 0);
        assert_eq!(video_data.dts, 0);
        assert_eq!(video_data.pts, 0);
        assert!(video_data.data.is_empty());
    }

    #[test]
    fn test_flv_demuxer_video_data_default() {
        let video_data = FlvDemuxerVideoData::default();
        assert_eq!(video_data.frame_type, 0);
        assert_eq!(video_data.codec_id, 0);
    }

    // ============================================
    // FlvDemuxer Construction Tests
    // ============================================

    #[test]
    fn test_flv_demuxer_new() {
        let data = BytesMut::new();
        let demuxer = FlvDemuxer::new(data);
        assert_eq!(demuxer.bytes_reader.len(), 0);
    }

    #[test]
    fn test_flv_demuxer_new_with_data() {
        let mut data = BytesMut::new();
        data.extend_from_slice(b"FLV");
        let demuxer = FlvDemuxer::new(data);
        assert_eq!(demuxer.bytes_reader.len(), 3);
    }

    // ============================================
    // FLV Header Reading Tests
    // ============================================

    #[test]
    fn test_read_flv_header_audio_video() {
        let mut muxer = FlvMuxer::new();
        muxer.write_flv_header(true, true).unwrap();
        let muxed_data = muxer.writer.get_current_bytes();

        let mut demuxer = FlvDemuxer::new(muxed_data);
        let result = demuxer.read_flv_header();
        assert!(result.is_ok());
    }

    #[test]
    fn test_read_flv_header_not_enough_bytes() {
        let mut data = BytesMut::new();
        data.extend_from_slice(&[0x46, 0x4C, 0x56]); // Only "FLV", incomplete
        let mut demuxer = FlvDemuxer::new(data);
        let result = demuxer.read_flv_header();
        assert!(result.is_err());
    }

    // ============================================
    // FLV Tag Reading Tests
    // ============================================

    #[test]
    fn test_read_flv_tag_audio() {
        let mut muxer = FlvMuxer::new();
        muxer.write_previous_tag_size(0).unwrap();

        let mut body = BytesMut::new();
        body.extend_from_slice(&[0xAF, 0x00, 0x10, 0x00]);
        let body_size = body.len() as u32;

        muxer.write_flv_tag_header(8, body_size, 0).unwrap();
        muxer.write_flv_tag_body(body).unwrap();
        muxer.write_previous_tag_size(11 + body_size).unwrap();

        let muxed_data = muxer.writer.get_current_bytes();
        let mut demuxer = FlvDemuxer::new(muxed_data);

        let result = demuxer.read_flv_tag();
        assert!(result.is_ok());
        if let Some(FlvData::Audio { timestamp, data }) = result.unwrap() {
            assert_eq!(timestamp, 0);
            assert_eq!(data.len(), 4);
        } else {
            panic!("Expected audio tag");
        }
    }

    #[test]
    fn test_read_flv_tag_video() {
        let mut muxer = FlvMuxer::new();
        muxer.write_previous_tag_size(0).unwrap();

        let mut body = BytesMut::new();
        body.extend_from_slice(&[0x17, 0x00, 0x00, 0x00, 0x00]);
        let body_size = body.len() as u32;

        muxer.write_flv_tag_header(9, body_size, 1000).unwrap();
        muxer.write_flv_tag_body(body).unwrap();
        muxer.write_previous_tag_size(11 + body_size).unwrap();

        let muxed_data = muxer.writer.get_current_bytes();
        let mut demuxer = FlvDemuxer::new(muxed_data);

        let result = demuxer.read_flv_tag();
        assert!(result.is_ok());
        if let Some(FlvData::Video { timestamp, data }) = result.unwrap() {
            assert_eq!(timestamp, 1000);
            assert_eq!(data.len(), 5);
        } else {
            panic!("Expected video tag");
        }
    }

    #[test]
    fn test_read_flv_tag_extended_timestamp() {
        let mut muxer = FlvMuxer::new();
        muxer.write_previous_tag_size(0).unwrap();

        let mut body = BytesMut::new();
        body.extend_from_slice(&[0x17, 0x00]);
        let body_size = body.len() as u32;

        // Timestamp > 24 bits
        let timestamp = 0x01000000;
        muxer.write_flv_tag_header(9, body_size, timestamp).unwrap();
        muxer.write_flv_tag_body(body).unwrap();
        muxer.write_previous_tag_size(11 + body_size).unwrap();

        let muxed_data = muxer.writer.get_current_bytes();
        let mut demuxer = FlvDemuxer::new(muxed_data);

        let result = demuxer.read_flv_tag();
        assert!(result.is_ok());
        if let Some(FlvData::Video {
            timestamp: read_timestamp,
            ..
        }) = result.unwrap()
        {
            assert_eq!(read_timestamp, timestamp);
        } else {
            panic!("Expected video tag");
        }
    }

    #[test]
    fn test_read_flv_tag_unknown_type() {
        let mut muxer = FlvMuxer::new();
        muxer.write_previous_tag_size(0).unwrap();

        let mut body = BytesMut::new();
        body.extend_from_slice(&[0x00]);
        let body_size = body.len() as u32;

        // Unknown tag type (not 8 or 9)
        muxer.write_flv_tag_header(18, body_size, 0).unwrap();
        muxer.write_flv_tag_body(body).unwrap();
        muxer.write_previous_tag_size(11 + body_size).unwrap();

        let muxed_data = muxer.writer.get_current_bytes();
        let mut demuxer = FlvDemuxer::new(muxed_data);

        let result = demuxer.read_flv_tag();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // Unknown tag type returns None
    }

    // ============================================
    // Round-trip Tests (Mux -> Demux)
    // ============================================

    #[test]
    fn test_mux_demux_roundtrip_audio() {
        let mut muxer = FlvMuxer::new();
        muxer.write_flv_header(true, false).unwrap();
        muxer.write_previous_tag_size(0).unwrap();

        let mut body = BytesMut::new();
        body.extend_from_slice(&[0xAF, 0x00, 0x10, 0x00, 0x11, 0x22]);
        let body_size = body.len() as u32;
        let timestamp = 1000;

        muxer.write_flv_tag_header(8, body_size, timestamp).unwrap();
        muxer.write_flv_tag_body(body.clone()).unwrap();
        muxer.write_previous_tag_size(11 + body_size).unwrap();

        let muxed_data = muxer.writer.get_current_bytes();
        let mut demuxer = FlvDemuxer::new(muxed_data);

        demuxer.read_flv_header().unwrap();
        let result = demuxer.read_flv_tag().unwrap();

        if let Some(FlvData::Audio {
            timestamp: read_timestamp,
            data: read_data,
        }) = result
        {
            assert_eq!(read_timestamp, timestamp);
            assert_eq!(&read_data[..], &body[..]);
        } else {
            panic!("Expected audio tag");
        }
    }

    #[test]
    fn test_mux_demux_roundtrip_video() {
        let mut muxer = FlvMuxer::new();
        muxer.write_flv_header(false, true).unwrap();
        muxer.write_previous_tag_size(0).unwrap();

        let mut body = BytesMut::new();
        body.extend_from_slice(&[0x17, 0x00, 0x00, 0x00, 0x00, 0x11, 0x22, 0x33]);
        let body_size = body.len() as u32;
        let timestamp = 2000;

        muxer.write_flv_tag_header(9, body_size, timestamp).unwrap();
        muxer.write_flv_tag_body(body.clone()).unwrap();
        muxer.write_previous_tag_size(11 + body_size).unwrap();

        let muxed_data = muxer.writer.get_current_bytes();
        let mut demuxer = FlvDemuxer::new(muxed_data);

        demuxer.read_flv_header().unwrap();
        let result = demuxer.read_flv_tag().unwrap();

        if let Some(FlvData::Video {
            timestamp: read_timestamp,
            data: read_data,
        }) = result
        {
            assert_eq!(read_timestamp, timestamp);
            assert_eq!(&read_data[..], &body[..]);
        } else {
            panic!("Expected video tag");
        }
    }

    #[test]
    fn test_timestamp_reconstruction_32_bit() {
        let test_timestamps = vec![0x01000000, 0x7FFFFFFF, 0xFFFFFFFF];

        for timestamp in test_timestamps {
            let mut muxer = FlvMuxer::new();
            muxer.write_previous_tag_size(0).unwrap();

            let mut body = BytesMut::new();
            body.extend_from_slice(&[0x17, 0x00]);
            let body_size = body.len() as u32;

            muxer.write_flv_tag_header(9, body_size, timestamp).unwrap();
            muxer.write_flv_tag_body(body).unwrap();
            muxer.write_previous_tag_size(11 + body_size).unwrap();

            let muxed_data = muxer.writer.get_current_bytes();
            let mut demuxer = FlvDemuxer::new(muxed_data);

            let result = demuxer.read_flv_tag().unwrap();
            if let Some(FlvData::Video {
                timestamp: read_timestamp,
                ..
            }) = result
            {
                assert_eq!(read_timestamp, timestamp);
            } else {
                panic!("Expected video tag");
            }
        }
    }

    // ============================================
    // FlvVideoTagDemuxer Tests
    // ============================================

    #[test]
    fn test_flv_video_tag_demuxer_seq_header() {
        let mut demuxer = FlvVideoTagDemuxer::new();
        // FrameType: 1 (Keyframe), CodecID: 7 (AVC) -> 0x17
        // AVCPacketType: 0 (SeqHdr)
        // CompositionTime: 0
        let mut data = BytesMut::new();
        data.extend_from_slice(&[0x17, 0x00, 0x00, 0x00, 0x00]);
        // Add AVCDecoderConfigurationRecord with 0 SPS and 0 PPS to avoid SpsParser errors with dummy data
        data.extend_from_slice(&[
            0x01, // Version
            0x42, // Profile
            0xC0, // Compatibility
            0x1E, // Level
            0xFF, // NALU Length Size - 1 (3 -> 4 bytes)
            0xE0, // Number of SPS = 0 (0xE0 | 0)
            0x00, // Number of PPS = 0
        ]);

        let result = demuxer.demux(0, data).unwrap();
        assert!(result.is_none()); // SeqHdr returns None but loads config

        // Verify config loaded
        assert_eq!(demuxer.avc_processor.mpeg4_avc.profile, 0x42);
        assert_eq!(demuxer.avc_processor.mpeg4_avc.nalu_length, 4);
    }

    #[test]
    fn test_flv_video_tag_demuxer_nalu() {
        let mut demuxer = FlvVideoTagDemuxer::new();
        // Set nalu_length explicitly as defaults might be 0
        demuxer.avc_processor.mpeg4_avc.nalu_length = 4;

        // FrameType: 2 (Interframe), CodecID: 7 (AVC) -> 0x27
        // AVCPacketType: 1 (NALU)
        // CompositionTime: 100
        let mut data = BytesMut::new();
        data.extend_from_slice(&[0x27, 0x01, 0x00, 0x00, 0x64]);

        // Add NALU: Size 4 bytes (e.g., 2), Data 2 bytes
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x02, 0x06, 0x05]);

        let result = demuxer.demux(1000, data).unwrap();
        assert!(result.is_some());
        let video_data = result.unwrap();
        assert_eq!(video_data.pts, 1100); // 1000 + 100
        assert_eq!(video_data.frame_type, 2);
        // Data should be Annex-B (0x00 0x00 0x00 0x01 ...)
        assert_eq!(video_data.data[0..4], [0x00, 0x00, 0x00, 0x01]);
        assert_eq!(video_data.data[4..6], [0x06, 0x05]);
    }

    // ============================================
    // FlvAudioTagDemuxer Tests
    // ============================================

    #[test]
    fn test_flv_audio_tag_demuxer_seq_header() {
        let mut demuxer = FlvAudioTagDemuxer::new();
        // AAC (10, 0xA), 44k(3), 16bit(1), Stereo(1) -> 0xAF
        // AACPacketType: 0 (SeqHdr)
        let mut data = BytesMut::new();
        data.extend_from_slice(&[0xAF, 0x00]);
        // AudioSpecificConfig (2 bytes usually, e.g. 0x12 0x10)
        data.extend_from_slice(&[0x12, 0x10]);

        let result = demuxer.demux(0, data).unwrap();
        assert!(!result.has_data); // SeqHdr returns empty data but loads config
    }

    #[test]
    fn test_flv_audio_tag_demuxer_raw() {
        let mut demuxer = FlvAudioTagDemuxer::new();
        // Pre-configure AAC processor to avoid overflow in adts_save due to default profile=0
        demuxer.aac_processor.mpeg4_aac.object_type = 2; // AAC LC
        demuxer.aac_processor.mpeg4_aac.sampling_frequency_index = 4; // 44100Hz
        demuxer.aac_processor.mpeg4_aac.channel_configuration = 2; // Stereo

        // AAC (10, 0xA) ... -> 0xAF
        // AACPacketType: 1 (Raw)
        let mut data = BytesMut::new();
        data.extend_from_slice(&[0xAF, 0x01]);
        // Raw data
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC]);

        let result = demuxer.demux(1000, data).unwrap();
        assert!(result.has_data);
        assert_eq!(result.pts, 1000);
        // Check that data contains raw data
        assert!(result.data.len() > 3);
    }
}
