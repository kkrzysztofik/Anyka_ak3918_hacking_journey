use {
    super::{define::h264_nal_type, errors::Mpeg4AvcHevcError},
    crate::bytesio::{bytes_reader::BytesReader, bytes_writer::BytesWriter},
    byteorder::BigEndian,
    bytes::BytesMut,
    std::vec::Vec,
};

use super::errors::MpegErrorValue;
use crate::codec::sps::SpsParser;

const H264_START_CODE: [u8; 4] = [0x00, 0x00, 0x00, 0x01];

#[derive(Clone, Default)]
pub struct Sps {
    // pub size: u16,
    pub data: BytesMut,
}

impl Sps {
    pub fn new() -> Self {
        Self {
            // size: 0,
            data: BytesMut::new(),
        }
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone, Default)]
pub struct Pps {
    // pub size: u16,
    pub data: BytesMut,
}

impl Pps {
    pub fn new() -> Self {
        Self {
            // size: 0,
            data: BytesMut::new(),
        }
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Default)]
pub struct Mpeg4Avc {
    pub profile: u8,
    pub compatibility: u8,
    pub level: u8,
    pub nalu_length: u8,
    pub width: u32,
    pub height: u32,

    pub nb_sps: u8,
    pub nb_pps: u8,

    pub sps: Vec<Sps>,
    pub pps: Vec<Pps>,

    pub sps_annexb_data: BytesWriter, // pice together all the sps data
    pub pps_annexb_data: BytesWriter, // pice together all the pps data

    //extension
    pub chroma_format_idc: u8,
    pub bit_depth_luma_minus8: u8,
    pub bit_depth_chroma_minus8: u8,
    // data: Vec<u8>, //[u8; 4 * 1024],
    // off: i32,
}

impl Mpeg4Avc {
    pub fn new() -> Self {
        Self {
            profile: 0,
            compatibility: 0,
            level: 0,
            nalu_length: 0,
            width: 0,
            height: 0,

            nb_pps: 0,
            nb_sps: 0,

            sps: Vec::new(),
            pps: Vec::new(),

            sps_annexb_data: BytesWriter::new(),
            pps_annexb_data: BytesWriter::new(),

            chroma_format_idc: 0,
            bit_depth_chroma_minus8: 0,
            bit_depth_luma_minus8: 0,
        }
    }
}

#[derive(Default)]
pub struct Mpeg4AvcProcessor {
    pub mpeg4_avc: Mpeg4Avc,
}

impl Mpeg4AvcProcessor {
    pub fn new() -> Self {
        Self {
            mpeg4_avc: Mpeg4Avc::new(),
        }
    }

    pub fn clear_sps_data(&mut self) {
        self.mpeg4_avc.sps.clear();
        self.mpeg4_avc.sps_annexb_data.clear();
    }

    pub fn clear_pps_data(&mut self) {
        self.mpeg4_avc.pps.clear();
        self.mpeg4_avc.pps_annexb_data.clear();
    }

    pub fn decoder_configuration_record_load(
        &mut self,
        bytes_reader: &mut BytesReader,
    ) -> Result<&mut Self, Mpeg4AvcHevcError> {
        /*version */
        bytes_reader.read_u8()?;
        /*avc profile*/
        self.mpeg4_avc.profile = bytes_reader.read_u8()?;
        /*avc compatibility*/
        self.mpeg4_avc.compatibility = bytes_reader.read_u8()?;
        /*avc level*/
        self.mpeg4_avc.level = bytes_reader.read_u8()?;
        /*nalu length*/
        self.mpeg4_avc.nalu_length = (bytes_reader.read_u8()? & 0x03) + 1;

        /*number of SPS NALUs */
        self.mpeg4_avc.nb_sps = bytes_reader.read_u8()? & 0x1F;

        if self.mpeg4_avc.nb_sps > 0 {
            self.clear_sps_data();
        }

        for i in 0..self.mpeg4_avc.nb_sps as usize {
            /*SPS size*/
            let sps_data_size = bytes_reader.read_u16::<BigEndian>()?;
            let sps_data = Sps {
                // size: sps_data_size,
                /*SPS data*/
                data: bytes_reader.read_bytes(sps_data_size as usize)?,
            };

            let mut sps_reader = BytesReader::new(sps_data.clone().data);
            /*parse SPS data to get video resolution(widthxheight) */
            let nal_type = sps_reader.read_u8()?;
            if (nal_type & 0x1f) != h264_nal_type::H264_NAL_SPS {
                return Err(Mpeg4AvcHevcError {
                    value: MpegErrorValue::SPSNalunitTypeNotCorrect,
                });
            }
            let mut sps_parser = SpsParser::new(sps_reader);
            (self.mpeg4_avc.width, self.mpeg4_avc.height) = sps_parser.parse()?;

            log::info!("mpeg4 avc profile: {}", self.mpeg4_avc.profile);
            log::info!("mpeg4 avc compatibility: {}", self.mpeg4_avc.compatibility);
            log::info!("mpeg4 avc level: {}", self.mpeg4_avc.level);
            log::info!(
                "mpeg4 avc resolution: {}x{}",
                self.mpeg4_avc.width,
                self.mpeg4_avc.height
            );

            self.mpeg4_avc.sps.push(sps_data);
            self.mpeg4_avc.sps_annexb_data.write(&H264_START_CODE)?;
            self.mpeg4_avc
                .sps_annexb_data
                .write(&self.mpeg4_avc.sps[i].data[..])?;
        }
        /*number of PPS NALUs*/
        self.mpeg4_avc.nb_pps = bytes_reader.read_u8()?;

        if self.mpeg4_avc.nb_pps > 0 {
            self.clear_pps_data();
        }

        for i in 0..self.mpeg4_avc.nb_pps as usize {
            let pps_data_size = bytes_reader.read_u16::<BigEndian>()?;
            let pps_data = Pps {
                // size: pps_data_size,
                data: bytes_reader.read_bytes(pps_data_size as usize)?,
            };

            self.mpeg4_avc.pps.push(pps_data);
            self.mpeg4_avc.pps_annexb_data.write(&H264_START_CODE)?;
            self.mpeg4_avc
                .pps_annexb_data
                .write(&self.mpeg4_avc.pps[i].data[..])?;
        }
        /*clear the left bytes*/
        bytes_reader.extract_remaining_bytes();

        Ok(self)
    }
    //https://stackoverflow.com/questions/28678615/efficiently-insert-or-replace-multiple-elements-in-the-middle-or-at-the-beginnin
    pub fn h264_mp4toannexb(
        &mut self,
        bytes_reader: &mut BytesReader,
    ) -> Result<BytesMut, Mpeg4AvcHevcError> {
        let mut bytes_writer = BytesWriter::new();

        let mut sps_pps_flag = false;
        while !bytes_reader.is_empty() {
            let size = self.read_nalu_size(bytes_reader)?;
            let nalu_type = bytes_reader.advance_u8()? & 0x1f;

            match nalu_type {
                h264_nal_type::H264_NAL_PPS | h264_nal_type::H264_NAL_SPS => {
                    sps_pps_flag = true;
                }
                h264_nal_type::H264_NAL_IDR => {
                    if !sps_pps_flag {
                        sps_pps_flag = true;

                        bytes_writer
                            .prepend(&self.mpeg4_avc.pps_annexb_data.get_current_bytes()[..])?;
                        bytes_writer
                            .prepend(&self.mpeg4_avc.sps_annexb_data.get_current_bytes()[..])?;
                    }
                }
                _ => {}
            }

            bytes_writer.write(&H264_START_CODE)?;
            let data = bytes_reader.read_bytes(size as usize)?;
            bytes_writer.write(&data[..])?;
        }

        Ok(bytes_writer.extract_current_bytes())
    }

    pub fn read_nalu_size(
        &mut self,
        bytes_reader: &mut BytesReader,
    ) -> Result<u32, Mpeg4AvcHevcError> {
        let mut size: u32 = 0;

        for _ in 0..self.mpeg4_avc.nalu_length {
            size = bytes_reader.read_u8()? as u32 + (size << 8);
        }
        Ok(size)
    }

    pub fn write_nalu_size(
        &mut self,
        writer: &mut BytesWriter,
        length: usize,
    ) -> Result<(), Mpeg4AvcHevcError> {
        let nalu_length = self.mpeg4_avc.nalu_length;
        for i in 0..nalu_length {
            let shift = (nalu_length - i - 1) * 8;
            let num = ((length >> shift) & 0xFF) as u8;
            writer.write_u8(num)?;
        }
        Ok(())
    }

    pub fn nalus_to_mpeg4avc(
        &mut self,
        nalus: Vec<BytesMut>,
    ) -> Result<BytesMut, Mpeg4AvcHevcError> {
        let mut bytes_writer = BytesWriter::new();

        for nalu in nalus {
            let length = nalu.len();
            self.write_nalu_size(&mut bytes_writer, length)?;
            bytes_writer.write(&nalu)?;
        }

        Ok(bytes_writer.extract_current_bytes())
    }

    pub fn decoder_configuration_record_save(&mut self) -> Result<BytesMut, Mpeg4AvcHevcError> {
        let mut bytes_writer = BytesWriter::new();

        bytes_writer.write_u8(1)?;
        bytes_writer.write_u8(self.mpeg4_avc.profile)?;
        bytes_writer.write_u8(self.mpeg4_avc.compatibility)?;
        bytes_writer.write_u8(self.mpeg4_avc.level)?;
        bytes_writer.write_u8((self.mpeg4_avc.nalu_length - 1) | 0xFC)?;

        //sps
        let sps_count = self.mpeg4_avc.sps.len() as u8;
        bytes_writer.write_u8((sps_count & 0x1F) | 0xE0)?;
        for sps in &self.mpeg4_avc.sps {
            bytes_writer.write_u16::<BigEndian>(sps.len() as u16)?;
            bytes_writer.write(&sps.data[..])?;
        }

        //pps
        let pps_count = self.mpeg4_avc.pps.len() as u8;
        bytes_writer.write_u8(pps_count)?;
        for pps in &self.mpeg4_avc.pps {
            bytes_writer.write_u16::<BigEndian>(pps.len() as u16)?;
            bytes_writer.write(&pps.data[..])?
        }

        match self.mpeg4_avc.profile {
            100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 => {
                bytes_writer.write_u8(0xFC | self.mpeg4_avc.chroma_format_idc)?;
                bytes_writer.write_u8(0xF8 | self.mpeg4_avc.bit_depth_luma_minus8)?;
                bytes_writer.write_u8(0xF8 | self.mpeg4_avc.bit_depth_chroma_minus8)?;
                bytes_writer.write_u8(0)?;
            }
            _ => {}
        }

        Ok(bytes_writer.extract_current_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytesio::{bytes_reader::BytesReader, bytes_writer::BytesWriter};
    use bytes::BytesMut;

    // ========== Constant Tests ==========

    #[test]
    fn test_h264_start_code() {
        assert_eq!(H264_START_CODE, [0x00, 0x00, 0x00, 0x01]);
    }

    // ========== Sps Tests ==========

    #[test]
    fn test_sps_new() {
        let sps = Sps::new();
        assert!(sps.data.is_empty());
        assert_eq!(sps.len(), 0);
        assert!(sps.is_empty());
    }

    #[test]
    fn test_sps_default() {
        let sps = Sps::default();
        assert!(sps.is_empty());
    }

    #[test]
    fn test_sps_with_data() {
        let mut sps = Sps::new();
        sps.data.extend_from_slice(&[0x67, 0x42, 0x00, 0x1e]);
        assert_eq!(sps.len(), 4);
        assert!(!sps.is_empty());
    }

    #[test]
    fn test_sps_clone() {
        let mut sps = Sps::new();
        sps.data.extend_from_slice(&[0x67, 0x42]);
        let cloned = sps.clone();
        assert_eq!(cloned.data, sps.data);
    }

    // ========== Pps Tests ==========

    #[test]
    fn test_pps_new() {
        let pps = Pps::new();
        assert!(pps.data.is_empty());
        assert_eq!(pps.len(), 0);
        assert!(pps.is_empty());
    }

    #[test]
    fn test_pps_default() {
        let pps = Pps::default();
        assert!(pps.is_empty());
    }

    #[test]
    fn test_pps_with_data() {
        let mut pps = Pps::new();
        pps.data.extend_from_slice(&[0x68, 0xce, 0x3c, 0x80]);
        assert_eq!(pps.len(), 4);
        assert!(!pps.is_empty());
    }

    #[test]
    fn test_pps_clone() {
        let mut pps = Pps::new();
        pps.data.extend_from_slice(&[0x68, 0xce]);
        let cloned = pps.clone();
        assert_eq!(cloned.data, pps.data);
    }

    // ========== Mpeg4Avc Tests ==========

    #[test]
    fn test_mpeg4_avc_new() {
        let avc = Mpeg4Avc::new();
        assert_eq!(avc.profile, 0);
        assert_eq!(avc.compatibility, 0);
        assert_eq!(avc.level, 0);
        assert_eq!(avc.nalu_length, 0);
        assert_eq!(avc.width, 0);
        assert_eq!(avc.height, 0);
        assert_eq!(avc.nb_sps, 0);
        assert_eq!(avc.nb_pps, 0);
        assert!(avc.sps.is_empty());
        assert!(avc.pps.is_empty());
    }

    #[test]
    fn test_mpeg4_avc_default() {
        let avc = Mpeg4Avc::default();
        assert_eq!(avc.profile, 0);
        assert!(avc.sps.is_empty());
    }

    // ========== Mpeg4AvcProcessor Tests ==========

    #[test]
    fn test_mpeg4_avc_processor_new() {
        let processor = Mpeg4AvcProcessor::new();
        assert_eq!(processor.mpeg4_avc.profile, 0);
    }

    #[test]
    fn test_mpeg4_avc_processor_default() {
        let processor = Mpeg4AvcProcessor::default();
        assert_eq!(processor.mpeg4_avc.profile, 0);
    }

    #[test]
    fn test_clear_sps_data() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.sps.push(Sps::new());
        processor
            .mpeg4_avc
            .sps_annexb_data
            .write(&[0x00, 0x00, 0x00, 0x01])
            .unwrap();

        processor.clear_sps_data();

        assert!(processor.mpeg4_avc.sps.is_empty());
    }

    #[test]
    fn test_clear_pps_data() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.pps.push(Pps::new());
        processor
            .mpeg4_avc
            .pps_annexb_data
            .write(&[0x00, 0x00, 0x00, 0x01])
            .unwrap();

        processor.clear_pps_data();

        assert!(processor.mpeg4_avc.pps.is_empty());
    }

    // ========== NALU Size Read/Write Tests ==========

    #[test]
    fn test_read_nalu_size_4_bytes() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 4;

        let data = BytesMut::from(&[0x00, 0x00, 0x03, 0xe8][..]);
        let mut reader = BytesReader::new(data);

        let size = processor.read_nalu_size(&mut reader).unwrap();
        assert_eq!(size, 1000);
    }

    #[test]
    fn test_read_nalu_size_2_bytes() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 2;

        let data = BytesMut::from(&[0x03, 0xe8][..]);
        let mut reader = BytesReader::new(data);

        let size = processor.read_nalu_size(&mut reader).unwrap();
        assert_eq!(size, 1000);
    }

    #[test]
    fn test_write_nalu_size_4_bytes() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 4;

        let mut writer = BytesWriter::new();
        processor.write_nalu_size(&mut writer, 1000).unwrap();

        let bytes = writer.extract_current_bytes();
        assert_eq!(bytes.as_ref(), &[0x00, 0x00, 0x03, 0xe8]);
    }

    #[test]
    fn test_write_nalu_size_2_bytes() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 2;

        let mut writer = BytesWriter::new();
        processor.write_nalu_size(&mut writer, 1000).unwrap();

        let bytes = writer.extract_current_bytes();
        assert_eq!(bytes.as_ref(), &[0x03, 0xe8]);
    }

    #[test]
    fn test_nalu_size_roundtrip() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 4;

        let original_size = 12345usize;

        let mut writer = BytesWriter::new();
        processor
            .write_nalu_size(&mut writer, original_size)
            .unwrap();

        let mut reader = BytesReader::new(writer.extract_current_bytes());
        let read_size = processor.read_nalu_size(&mut reader).unwrap();

        assert_eq!(read_size, original_size as u32);
    }

    // ========== nalus_to_mpeg4avc Tests ==========

    #[test]
    fn test_nalus_to_mpeg4avc_single_nalu() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 4;

        let nalu = BytesMut::from(&[0x65, 0x88, 0x84, 0x00][..]);
        let nalus = vec![nalu];

        let result = processor.nalus_to_mpeg4avc(nalus).unwrap();

        // Should have 4 bytes length prefix + 4 bytes data
        assert_eq!(result.len(), 8);
        // Length prefix for 4 bytes
        assert_eq!(&result[0..4], &[0x00, 0x00, 0x00, 0x04]);
        // Data
        assert_eq!(&result[4..8], &[0x65, 0x88, 0x84, 0x00]);
    }

    #[test]
    fn test_nalus_to_mpeg4avc_multiple_nalus() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 4;

        let nalu1 = BytesMut::from(&[0x67, 0x42][..]);
        let nalu2 = BytesMut::from(&[0x68, 0xce][..]);
        let nalus = vec![nalu1, nalu2];

        let result = processor.nalus_to_mpeg4avc(nalus).unwrap();

        // Should have (4+2) + (4+2) = 12 bytes
        assert_eq!(result.len(), 12);
    }

    // ========== decoder_configuration_record_save Tests ==========

    #[test]
    fn test_decoder_configuration_record_save_basic() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.profile = 66; // Baseline
        processor.mpeg4_avc.compatibility = 0;
        processor.mpeg4_avc.level = 30; // Level 3.0
        processor.mpeg4_avc.nalu_length = 4;
        processor.mpeg4_avc.nb_sps = 0;
        processor.mpeg4_avc.nb_pps = 0;

        let result = processor.decoder_configuration_record_save().unwrap();

        // Should have at least version, profile, compat, level, nalu_length, nb_sps, nb_pps
        assert!(result.len() >= 7);
        assert_eq!(result[0], 1); // version
        assert_eq!(result[1], 66); // profile
        assert_eq!(result[2], 0); // compatibility
        assert_eq!(result[3], 30); // level
        assert_eq!(result[4], 0xFF); // nalu_length - 1 | 0xFC = 3 | 0xFC = 0xFF
    }

    // ========== Original Tests ==========

    #[test]
    fn test_bytes_to_bigend() {
        let mut size: u32 = 0;
        let mut b = BytesMut::new();
        b.extend_from_slice(b"\0\0\x03\xe8");
        let mut bytes_reader = BytesReader::new(b);

        for _ in 0..4 {
            size = bytes_reader.read_u8().unwrap() as u32 + (size << 8);
        }
        assert_eq!(size, 1000);
    }

    #[test]
    fn test_bigend_to_bytes() {
        let size = 1000;
        let length = 4;
        let mut bytes_writer = BytesWriter::new();

        for i in 0..length {
            let shift = (length - i - 1) * 8;
            let num = ((size >> shift) & 0xFF) as u8;
            bytes_writer.write_u8(num).unwrap();
        }
        let result = bytes_writer.extract_current_bytes();
        assert_eq!(result.as_ref(), &[0x00, 0x00, 0x03, 0xe8]);
    }

    // ========== Mpeg4AvcProcessor Tests ==========

    #[test]
    fn test_mpeg4_avc_processor_initial_state() {
        let processor = Mpeg4AvcProcessor::new();
        assert!(processor.mpeg4_avc.sps.is_empty());
        assert!(processor.mpeg4_avc.pps.is_empty());
    }

    #[test]
    fn test_mpeg4_avc_processor_clear_data() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.sps.push(Sps {
            data: BytesMut::from(&[1, 2, 3][..]),
        });
        processor.mpeg4_avc.pps.push(Pps {
            data: BytesMut::from(&[4, 5, 6][..]),
        });

        processor.clear_sps_data();
        assert!(processor.mpeg4_avc.sps.is_empty());
        // PPS should still be there
        assert!(!processor.mpeg4_avc.pps.is_empty());

        processor.clear_pps_data();
        assert!(processor.mpeg4_avc.pps.is_empty());
    }

    #[test]
    fn test_nalus_to_mpeg4avc() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 4;

        let nal1 = BytesMut::from(&[0x06, 0x01][..]); // SEI
        let nal2 = BytesMut::from(&[0x65, 0x02][..]); // IDR

        let nalus = vec![nal1.clone(), nal2.clone()];

        // Expected output: 4-byte size + NAL1 + 4-byte size + NAL2
        // Size 2 = 0x00 0x00 0x00 0x02
        let result = processor.nalus_to_mpeg4avc(nalus).unwrap();

        assert_eq!(result.len(), 4 + 2 + 4 + 2);
        assert_eq!(result[0..4], [0x00, 0x00, 0x00, 0x02]);
        assert_eq!(result[4..6], [0x06, 0x01]);
        assert_eq!(result[6..10], [0x00, 0x00, 0x00, 0x02]);
        assert_eq!(result[10..12], [0x65, 0x02]);
    }

    #[test]
    fn test_decoder_configuration_record_save() {
        let mut processor = Mpeg4AvcProcessor::new();
        // Add dummy SPS and PPS
        // SPS NAL type 7 (0x67)
        let sps_data = BytesMut::from(&[0x67, 0x42, 0xC0, 0x1E][..]);
        // PPS NAL type 8 (0x68)
        let pps_data = BytesMut::from(&[0x68, 0xCE, 0x3C, 0x80][..]);

        processor.mpeg4_avc.sps.push(Sps {
            data: sps_data.clone(),
        });
        processor.mpeg4_avc.pps.push(Pps {
            data: pps_data.clone(),
        });
        processor.mpeg4_avc.nb_sps = 1;
        processor.mpeg4_avc.nb_pps = 1;

        processor.mpeg4_avc.profile = 0x42; // Baseline
        processor.mpeg4_avc.compatibility = 0xC0;
        processor.mpeg4_avc.level = 0x1E;
        processor.mpeg4_avc.nalu_length = 4; // lengthSizeMinusOne = 3

        let record = processor.decoder_configuration_record_save().unwrap();

        // Check header
        assert_eq!(record[0], 0x01); // version
        assert_eq!(record[1], 0x42); // profile
        assert_eq!(record[2], 0xC0); // compatibility
        assert_eq!(record[3], 0x1E); // level
        assert_eq!(record[4] & 0xFC, 0xFC); // reserved
        assert_eq!(record[4] & 0x03, 0x03); // length size minus one

        // Check SPS count
        assert_eq!(record[5] & 0xE0, 0xE0); // reserved
        assert_eq!(record[5] & 0x1F, 0x01); // 1 SPS

        // Check SPS size (2 bytes)
        assert_eq!(record[6], 0x00);
        assert_eq!(record[7], 0x04);

        // Check SPS data
        assert_eq!(&record[8..12], &sps_data[..]);

        // Check PPS count
        assert_eq!(record[12], 0x01); // 1 PPS

        // Check PPS size
        assert_eq!(record[13], 0x00);
        assert_eq!(record[14], 0x04);

        // Check PPS data
        assert_eq!(&record[15..19], &pps_data[..]);
    }

    #[test]
    fn test_read_nalu_size() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 4;
        let data = BytesMut::from(&[0x00, 0x00, 0x01, 0x00][..]); // 256
        let mut reader = BytesReader::new(data);

        let size = processor.read_nalu_size(&mut reader).unwrap();
        assert_eq!(size, 256);
    }

    // ========== decoder_configuration_record_save High Profile Tests ==========

    #[test]
    fn test_decoder_configuration_record_save_high_profile_has_extension_data() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.profile = 100; // High profile
        processor.mpeg4_avc.compatibility = 0x00;
        processor.mpeg4_avc.level = 0x1F;
        processor.mpeg4_avc.nalu_length = 4;
        processor.mpeg4_avc.chroma_format_idc = 1;
        processor.mpeg4_avc.bit_depth_luma_minus8 = 0;
        processor.mpeg4_avc.bit_depth_chroma_minus8 = 0;

        let record = processor.decoder_configuration_record_save().unwrap();

        // Base: version(1) + profile(1) + compat(1) + level(1) + nalu_len(1) + sps_count(1) + pps_count(1) = 7
        // Extension for High: chroma_format(1) + bit_depth_luma(1) + bit_depth_chroma(1) + num_sps_ext(1) = 4
        assert_eq!(record.len(), 7 + 4);
        assert_eq!(record[0], 1); // version
        assert_eq!(record[1], 100); // High profile

        // Extension data checks
        assert_eq!(record[7] & 0xFC, 0xFC); // reserved bits
        assert_eq!(record[7] & 0x03, 1); // chroma_format_idc
        assert_eq!(record[8] & 0xF8, 0xF8); // reserved bits
        assert_eq!(record[8] & 0x07, 0); // bit_depth_luma_minus8
        assert_eq!(record[9] & 0xF8, 0xF8); // reserved bits
        assert_eq!(record[9] & 0x07, 0); // bit_depth_chroma_minus8
        assert_eq!(record[10], 0); // num SPS ext = 0
    }

    // ========== decoder_configuration_record_load Tests ==========

    #[test]
    fn test_decoder_configuration_record_load_baseline_720p() {
        let mut processor = Mpeg4AvcProcessor::new();

        // Build a minimal AVCDecoderConfigurationRecord with Baseline SPS for 1280x720
        let mut writer = BytesWriter::new();
        writer.write_u8(1).unwrap(); // version
        writer.write_u8(66).unwrap(); // Baseline profile
        writer.write_u8(0xC0).unwrap(); // compatibility
        writer.write_u8(30).unwrap(); // level 3.0
        writer.write_u8(0xFF).unwrap(); // nalu_length = 4 (3 | 0xFC)

        // 1 SPS
        writer.write_u8(0xE1).unwrap(); // 1 SPS (0x01 | 0xE0)
        // SPS: Baseline profile, 1280x720
        // Properly bit-packed exp-Golomb encoded SPS data
        let sps_data: &[u8] = &[
            0x67, // NAL type SPS
            0x42, // profile_idc = 66 (Baseline)
            0xC0, // constraint_set flags
            0x1E, // level_idc = 30
            0xDA, // UEV(0)+UEV(0)+UEV(2)+UEV(1)
            0x01, // gaps=0, start of UEV(79) for pic_width_in_mbs_minus1
            0x40, // middle of UEV(79), start of UEV(44) for pic_height_in_map_units_minus1
            0x16, // middle of UEV(44)
            0xE0, // end of UEV(44), frame_mbs_only=1, direct_8x8=1, cropping=0, vui=0
        ];
        writer
            .write_u16::<BigEndian>(sps_data.len() as u16)
            .unwrap();
        writer.write(sps_data).unwrap();

        // 1 PPS
        writer.write_u8(1).unwrap(); // 1 PPS
        let pps_data: &[u8] = &[0x68, 0xCE, 0x3C, 0x80];
        writer
            .write_u16::<BigEndian>(pps_data.len() as u16)
            .unwrap();
        writer.write(pps_data).unwrap();

        let mut reader = BytesReader::new(writer.extract_current_bytes());
        let result = processor.decoder_configuration_record_load(&mut reader);
        assert!(result.is_ok());

        assert_eq!(processor.mpeg4_avc.profile, 66);
        assert_eq!(processor.mpeg4_avc.compatibility, 0xC0);
        assert_eq!(processor.mpeg4_avc.level, 30);
        assert_eq!(processor.mpeg4_avc.nalu_length, 4);
        assert_eq!(processor.mpeg4_avc.nb_sps, 1);
        assert_eq!(processor.mpeg4_avc.nb_pps, 1);
        assert_eq!(processor.mpeg4_avc.sps.len(), 1);
        assert_eq!(processor.mpeg4_avc.pps.len(), 1);
        // Resolution should have been parsed from SPS
        assert!(processor.mpeg4_avc.width > 0);
        assert!(processor.mpeg4_avc.height > 0);
    }

    #[test]
    fn test_decoder_configuration_record_load_wrong_sps_nal_type_returns_error() {
        let mut processor = Mpeg4AvcProcessor::new();

        let mut writer = BytesWriter::new();
        writer.write_u8(1).unwrap(); // version
        writer.write_u8(66).unwrap(); // profile
        writer.write_u8(0).unwrap(); // compat
        writer.write_u8(30).unwrap(); // level
        writer.write_u8(0xFF).unwrap(); // nalu_length = 4

        // 1 SPS but with wrong NAL type (PPS type 8 instead of SPS type 7)
        writer.write_u8(0xE1).unwrap();
        let bad_sps_data: &[u8] = &[0x68, 0xCE, 0x3C, 0x80]; // PPS NAL type, not SPS
        writer
            .write_u16::<BigEndian>(bad_sps_data.len() as u16)
            .unwrap();
        writer.write(bad_sps_data).unwrap();

        let mut reader = BytesReader::new(writer.extract_current_bytes());
        let result = processor.decoder_configuration_record_load(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_decoder_configuration_record_load_no_sps_no_pps() {
        let mut processor = Mpeg4AvcProcessor::new();

        let mut writer = BytesWriter::new();
        writer.write_u8(1).unwrap(); // version
        writer.write_u8(66).unwrap(); // profile
        writer.write_u8(0).unwrap(); // compat
        writer.write_u8(30).unwrap(); // level
        writer.write_u8(0xFF).unwrap(); // nalu_length = 4

        // 0 SPS
        writer.write_u8(0xE0).unwrap(); // 0 SPS (0x00 | 0xE0)
        // 0 PPS
        writer.write_u8(0).unwrap();

        let mut reader = BytesReader::new(writer.extract_current_bytes());
        let result = processor.decoder_configuration_record_load(&mut reader);
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_avc.nb_sps, 0);
        assert_eq!(processor.mpeg4_avc.nb_pps, 0);
    }

    // ========== h264_mp4toannexb Tests ==========

    #[test]
    fn test_h264_mp4toannexb_single_sei_nalu() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 4;

        // Build MP4 data: 4-byte size + SEI NAL (type 6)
        let mut mp4_data = BytesMut::new();
        mp4_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x03]); // size=3
        mp4_data.extend_from_slice(&[0x06, 0x01, 0x02]); // SEI data

        let mut reader = BytesReader::new(mp4_data);
        let annexb = processor.h264_mp4toannexb(&mut reader).unwrap();

        // Should have start code (4) + data (3) = 7 bytes
        assert_eq!(annexb.len(), 7);
        assert_eq!(&annexb[..4], &H264_START_CODE);
        assert_eq!(annexb[4], 0x06); // SEI NAL type
    }

    #[test]
    fn test_h264_mp4toannexb_idr_with_sps_pps_prepend() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 4;

        // Preload SPS/PPS annexb data
        processor
            .mpeg4_avc
            .sps_annexb_data
            .write(&[0x00, 0x00, 0x00, 0x01, 0x67, 0x42])
            .unwrap();
        processor
            .mpeg4_avc
            .pps_annexb_data
            .write(&[0x00, 0x00, 0x00, 0x01, 0x68, 0xCE])
            .unwrap();

        // Build MP4 data with IDR NAL (type 5)
        let mut mp4_data = BytesMut::new();
        mp4_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x02]); // size=2
        mp4_data.extend_from_slice(&[0x65, 0x88]); // IDR slice

        let mut reader = BytesReader::new(mp4_data);
        let annexb = processor.h264_mp4toannexb(&mut reader).unwrap();

        // Should contain SPS annexb + PPS annexb + start code + IDR data
        // SPS: 6 bytes, PPS: 6 bytes, start code: 4 bytes, IDR: 2 bytes = 18
        assert_eq!(annexb.len(), 6 + 6 + 4 + 2);
    }

    #[test]
    fn test_h264_mp4toannexb_sps_pps_nalu_does_not_prepend() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 4;

        // Build MP4 data with SPS NAL (type 7) followed by IDR (type 5)
        let mut mp4_data = BytesMut::new();
        // SPS
        mp4_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x02]); // size=2
        mp4_data.extend_from_slice(&[0x67, 0x42]); // SPS
        // IDR (sps_pps_flag should already be true, so no prepend)
        mp4_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x02]); // size=2
        mp4_data.extend_from_slice(&[0x65, 0x88]); // IDR

        let mut reader = BytesReader::new(mp4_data);
        let annexb = processor.h264_mp4toannexb(&mut reader).unwrap();

        // Should have: start_code + SPS(2) + start_code + IDR(2) = 4+2+4+2 = 12
        assert_eq!(annexb.len(), 12);
    }

    // ========== decoder_configuration_record roundtrip ==========

    #[test]
    fn test_decoder_configuration_record_save_load_roundtrip() {
        let mut save_processor = Mpeg4AvcProcessor::new();
        save_processor.mpeg4_avc.profile = 66; // Baseline
        save_processor.mpeg4_avc.compatibility = 0xC0;
        save_processor.mpeg4_avc.level = 30;
        save_processor.mpeg4_avc.nalu_length = 4;

        // Create a minimal valid SPS (Baseline, 1280x720, properly bit-packed)
        let sps_data: &[u8] = &[0x67, 0x42, 0xC0, 0x1E, 0xDA, 0x01, 0x40, 0x16, 0xE0];
        save_processor.mpeg4_avc.sps.push(Sps {
            data: BytesMut::from(sps_data),
        });
        save_processor.mpeg4_avc.nb_sps = 1;

        let pps_data: &[u8] = &[0x68, 0xCE, 0x3C, 0x80];
        save_processor.mpeg4_avc.pps.push(Pps {
            data: BytesMut::from(pps_data),
        });
        save_processor.mpeg4_avc.nb_pps = 1;

        // Save
        let record = save_processor.decoder_configuration_record_save().unwrap();

        // Load
        let mut load_processor = Mpeg4AvcProcessor::new();
        let mut reader = BytesReader::new(record);
        load_processor
            .decoder_configuration_record_load(&mut reader)
            .unwrap();

        assert_eq!(load_processor.mpeg4_avc.profile, 66);
        assert_eq!(load_processor.mpeg4_avc.compatibility, 0xC0);
        assert_eq!(load_processor.mpeg4_avc.level, 30);
        assert_eq!(load_processor.mpeg4_avc.nalu_length, 4);
        assert_eq!(load_processor.mpeg4_avc.sps.len(), 1);
        assert_eq!(load_processor.mpeg4_avc.pps.len(), 1);
        assert_eq!(load_processor.mpeg4_avc.sps[0].data[..], sps_data[..]);
        assert_eq!(load_processor.mpeg4_avc.pps[0].data[..], pps_data[..]);
    }

    // ========== Edge Case Tests ==========

    #[test]
    fn test_h264_mp4toannexb_empty_input() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 4;

        // Empty BytesMut input
        let empty_data = BytesMut::new();
        let mut reader = BytesReader::new(empty_data);
        let result = processor.h264_mp4toannexb(&mut reader).unwrap();

        // Should return empty result
        assert!(result.is_empty());
    }

    #[test]
    fn test_decoder_configuration_record_load_minimal_valid_data() {
        let mut processor = Mpeg4AvcProcessor::new();

        // Build minimal valid AVC config record with smallest valid SPS
        let mut writer = BytesWriter::new();
        writer.write_u8(1).unwrap(); // version
        writer.write_u8(66).unwrap(); // Baseline profile
        writer.write_u8(0).unwrap(); // compatibility
        writer.write_u8(10).unwrap(); // level 1.0
        writer.write_u8(0xFF).unwrap(); // nalu_length = 4

        // Minimal SPS (properly encoded)
        writer.write_u8(0xE1).unwrap(); // 1 SPS
        let minimal_sps: &[u8] = &[0x67, 0x42, 0x00, 0x0A, 0xDA, 0x01, 0x40, 0x16, 0xE0];
        writer
            .write_u16::<BigEndian>(minimal_sps.len() as u16)
            .unwrap();
        writer.write(minimal_sps).unwrap();

        // Minimal PPS
        writer.write_u8(1).unwrap(); // 1 PPS
        let minimal_pps: &[u8] = &[0x68, 0xCE, 0x3C, 0x80];
        writer
            .write_u16::<BigEndian>(minimal_pps.len() as u16)
            .unwrap();
        writer.write(minimal_pps).unwrap();

        let mut reader = BytesReader::new(writer.extract_current_bytes());
        let result = processor.decoder_configuration_record_load(&mut reader);

        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_avc.profile, 66);
        assert_eq!(processor.mpeg4_avc.level, 10);
        assert_eq!(processor.mpeg4_avc.nb_sps, 1);
        assert_eq!(processor.mpeg4_avc.nb_pps, 1);
    }

    #[test]
    fn test_h264_mp4toannexb_empty_sps_pps_data() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 4;

        // Empty SPS/PPS annexb data (not preloaded)
        assert!(
            processor
                .mpeg4_avc
                .sps_annexb_data
                .get_current_bytes()
                .is_empty()
        );
        assert!(
            processor
                .mpeg4_avc
                .pps_annexb_data
                .get_current_bytes()
                .is_empty()
        );

        // Build MP4 data with IDR NAL (should attempt prepend but with empty data)
        let mut mp4_data = BytesMut::new();
        mp4_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x02]); // size=2
        mp4_data.extend_from_slice(&[0x65, 0x88]); // IDR slice

        let mut reader = BytesReader::new(mp4_data);
        let annexb = processor.h264_mp4toannexb(&mut reader).unwrap();

        // Should only have start code + IDR data (no SPS/PPS prepended since empty)
        // Result: 4 (start code) + 2 (IDR data) = 6 bytes
        assert_eq!(annexb.len(), 6);
        assert_eq!(&annexb[..4], &H264_START_CODE);
        assert_eq!(annexb[4], 0x65); // IDR NAL type
    }

    #[test]
    fn test_nalus_to_mpeg4avc_empty_vector() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 4;

        let empty_nalus: Vec<BytesMut> = vec![];
        let result = processor.nalus_to_mpeg4avc(empty_nalus).unwrap();

        // Should return empty result
        assert!(result.is_empty());
    }

    #[test]
    fn test_decoder_configuration_record_load_multiple_sps_pps() {
        let mut processor = Mpeg4AvcProcessor::new();

        let mut writer = BytesWriter::new();
        writer.write_u8(1).unwrap(); // version
        writer.write_u8(66).unwrap(); // profile
        writer.write_u8(0).unwrap(); // compat
        writer.write_u8(30).unwrap(); // level
        writer.write_u8(0xFF).unwrap(); // nalu_length

        // 2 SPS entries
        writer.write_u8(0xE2).unwrap(); // 2 SPS (0x02 | 0xE0)
        let sps1: &[u8] = &[0x67, 0x42, 0x00, 0x1E, 0xDA, 0x01, 0x40, 0x16, 0xE0];
        writer.write_u16::<BigEndian>(sps1.len() as u16).unwrap();
        writer.write(sps1).unwrap();

        let sps2: &[u8] = &[0x67, 0x42, 0x00, 0x1E, 0xDA, 0x01, 0x40, 0x16, 0xE0];
        writer.write_u16::<BigEndian>(sps2.len() as u16).unwrap();
        writer.write(sps2).unwrap();

        // 2 PPS entries
        writer.write_u8(2).unwrap(); // 2 PPS
        let pps1: &[u8] = &[0x68, 0xCE, 0x3C, 0x80];
        writer.write_u16::<BigEndian>(pps1.len() as u16).unwrap();
        writer.write(pps1).unwrap();

        let pps2: &[u8] = &[0x68, 0xCE, 0x3C, 0x81];
        writer.write_u16::<BigEndian>(pps2.len() as u16).unwrap();
        writer.write(pps2).unwrap();

        let mut reader = BytesReader::new(writer.extract_current_bytes());
        let result = processor.decoder_configuration_record_load(&mut reader);

        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_avc.nb_sps, 2);
        assert_eq!(processor.mpeg4_avc.nb_pps, 2);
        assert_eq!(processor.mpeg4_avc.sps.len(), 2);
        assert_eq!(processor.mpeg4_avc.pps.len(), 2);
    }

    #[test]
    fn test_read_nalu_size_1_byte() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 1;

        let data = BytesMut::from(&[0xFF][..]);
        let mut reader = BytesReader::new(data);

        let size = processor.read_nalu_size(&mut reader).unwrap();
        assert_eq!(size, 255);
    }

    #[test]
    fn test_write_nalu_size_1_byte() {
        let mut processor = Mpeg4AvcProcessor::new();
        processor.mpeg4_avc.nalu_length = 1;

        let mut writer = BytesWriter::new();
        processor.write_nalu_size(&mut writer, 255).unwrap();

        let bytes = writer.extract_current_bytes();
        assert_eq!(bytes.as_ref(), &[0xFF]);
    }
}
