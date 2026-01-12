use {
    super::errors::H264Error, super::utils, crate::bytesio::bits_reader::BitsReader,
    crate::bytesio::bytes_reader::BytesReader, bytes::BytesMut, std::vec::Vec,
};

#[derive(Default, Debug)]
pub struct Sps {
    pub profile_idc: u8, // u(8)
    flag: u8,

    pub level_idc: u8,         // u(8)
    seq_parameter_set_id: u32, // ue(v)

    chroma_format_idc: u32, // ue(v)

    separate_colour_plane_flag: u8,           // u(1)
    bit_depth_luma_minus8: u32,               // ue(v)
    bit_depth_chroma_minus8: u32,             // ue(v)
    qpprime_y_zero_transform_bypass_flag: u8, // u(1)

    seq_scaling_matrix_present_flag: u8, // u(1)

    seq_scaling_list_present_flag: Vec<u8>, // u(1)

    log2_max_frame_num_minus4: u32, // ue(v)
    pic_order_cnt_type: u32,        // ue(v)

    log2_max_pic_order_cnt_lsb_minus4: u32, // ue(v)

    delta_pic_order_always_zero_flag: u8,       // u(1)
    offset_for_non_ref_pic: i32,                // se(v)
    offset_for_top_to_bottom_field: i32,        // se(v)
    num_ref_frames_in_pic_order_cnt_cycle: u32, // ue(v)

    offset_for_ref_frame: Vec<i32>, // se(v)

    max_num_ref_frames: u32,                  // ue(v)
    gaps_in_frame_num_value_allowed_flag: u8, // u(1)

    pic_width_in_mbs_minus1: u32,        // ue(v)
    pic_height_in_map_units_minus1: u32, // ue(v)
    frame_mbs_only_flag: u8,             // u(1)

    mb_adaptive_frame_field_flag: u8, // u(1)

    direct_8x8_inference_flag: u8, // u(1)

    frame_cropping_flag: u8, // u(1)

    frame_crop_left_offset: u32,   // ue(v)
    frame_crop_right_offset: u32,  // ue(v)
    frame_crop_top_offset: u32,    // ue(v)
    frame_crop_bottom_offset: u32, // ue(v)

    vui_parameters_present_flag: u8, // u(1)
}

pub struct SpsParser {
    pub bytes_reader: BytesReader,
    pub bits_reader: BitsReader,
    pub sps: Sps,
}

impl SpsParser {
    pub fn new(reader: BytesReader) -> SpsParser {
        Self {
            bytes_reader: BytesReader::new(BytesMut::new()),
            bits_reader: BitsReader::new(reader),
            sps: Sps::default(),
        }
    }

    pub fn extend_data(&mut self, data: BytesMut) {
        self.bits_reader.extend_data(data);
    }

    pub fn parse(&mut self) -> Result<(u32, u32), H264Error> {
        self.sps.profile_idc = self.bits_reader.read_byte()?;
        log::info!("profile_idc: {}", self.sps.profile_idc);
        self.sps.flag = self.bits_reader.read_byte()?;
        self.sps.level_idc = self.bits_reader.read_byte()?;
        log::info!("level_idc: {}", self.sps.level_idc);
        self.sps.seq_parameter_set_id = utils::read_uev(&mut self.bits_reader)?;

        match self.sps.profile_idc {
            100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 => {
                self.sps.chroma_format_idc = utils::read_uev(&mut self.bits_reader)?;
                if self.sps.chroma_format_idc == 3 {
                    self.sps.separate_colour_plane_flag = self.bits_reader.read_bit()?;
                }
                self.sps.bit_depth_luma_minus8 = utils::read_uev(&mut self.bits_reader)?;
                self.sps.bit_depth_chroma_minus8 = utils::read_uev(&mut self.bits_reader)?;

                self.sps.qpprime_y_zero_transform_bypass_flag = self.bits_reader.read_bit()?;
                self.sps.seq_scaling_matrix_present_flag = self.bits_reader.read_bit()?;

                if self.sps.seq_scaling_matrix_present_flag > 0 {
                    let matrix_dim: usize = if self.sps.chroma_format_idc != 2 {
                        8
                    } else {
                        12
                    };

                    for _ in 0..matrix_dim {
                        self.sps
                            .seq_scaling_list_present_flag
                            .push(self.bits_reader.read_bit()?);
                    }
                }
            }
            _ => {
                // For profiles that don't include chroma_format_idc, default to 1 (4:2:0) per H.264 spec
                self.sps.chroma_format_idc = 1;
            }
        }

        self.sps.log2_max_frame_num_minus4 = utils::read_uev(&mut self.bits_reader)?;
        self.sps.pic_order_cnt_type = utils::read_uev(&mut self.bits_reader)?;

        match self.sps.pic_order_cnt_type {
            0 => {
                self.sps.log2_max_pic_order_cnt_lsb_minus4 =
                    utils::read_uev(&mut self.bits_reader)?;
            }
            1 => {
                self.sps.delta_pic_order_always_zero_flag = self.bits_reader.read_bit()?;
                self.sps.offset_for_non_ref_pic = utils::read_sev(&mut self.bits_reader)?;
                self.sps.offset_for_top_to_bottom_field = utils::read_sev(&mut self.bits_reader)?;
                self.sps.num_ref_frames_in_pic_order_cnt_cycle =
                    utils::read_uev(&mut self.bits_reader)?;

                self.sps.offset_for_ref_frame.clear();
                if self.sps.num_ref_frames_in_pic_order_cnt_cycle > 0 {
                    for _ in 0..self.sps.num_ref_frames_in_pic_order_cnt_cycle as usize {
                        self.sps
                            .offset_for_ref_frame
                            .push(utils::read_sev(&mut self.bits_reader)?);
                    }
                }
            }
            _ => {}
        }

        self.sps.max_num_ref_frames = utils::read_uev(&mut self.bits_reader)?;
        self.sps.gaps_in_frame_num_value_allowed_flag = self.bits_reader.read_bit()?;

        // Align to byte boundary before reading pic_width_in_mbs_minus1
        self.bits_reader.bits_aligment_8();
        self.sps.pic_width_in_mbs_minus1 = utils::read_uev(&mut self.bits_reader)?;
        // Align to byte boundary before reading pic_height_in_map_units_minus1
        self.bits_reader.bits_aligment_8();
        self.sps.pic_height_in_map_units_minus1 = utils::read_uev(&mut self.bits_reader)?;

        self.sps.frame_mbs_only_flag = self.bits_reader.read_bit()?;

        if self.sps.frame_mbs_only_flag == 0 {
            self.sps.mb_adaptive_frame_field_flag = self.bits_reader.read_bit()?;
        }
        self.sps.direct_8x8_inference_flag = self.bits_reader.read_bit()?;
        self.sps.frame_cropping_flag = self.bits_reader.read_bit()?;

        if self.sps.frame_cropping_flag > 0 {
            self.sps.frame_crop_left_offset = utils::read_uev(&mut self.bits_reader)?;
            self.sps.frame_crop_right_offset = utils::read_uev(&mut self.bits_reader)?;
            self.sps.frame_crop_top_offset = utils::read_uev(&mut self.bits_reader)?;
            self.sps.frame_crop_bottom_offset = utils::read_uev(&mut self.bits_reader)?;
        } else {
            // Explicitly set to 0 when flag is not set (should already be 0 from Default, but being explicit)
            self.sps.frame_crop_left_offset = 0;
            self.sps.frame_crop_right_offset = 0;
            self.sps.frame_crop_top_offset = 0;
            self.sps.frame_crop_bottom_offset = 0;
        }

        self.sps.vui_parameters_present_flag = self.bits_reader.read_bit()?;

        // Calculate width: only apply crop offsets if frame_cropping_flag is set
        let mut width = (self.sps.pic_width_in_mbs_minus1 + 1) * 16;
        if self.sps.frame_cropping_flag > 0 {
            // crop_unit_x = 2 for chroma_format_idc = 1 (4:2:0, default for Baseline)
            // For other chroma formats, this would need to be adjusted
            let crop_unit_x = if self.sps.chroma_format_idc == 0 {
                1
            } else {
                2
            };
            width -=
                (self.sps.frame_crop_left_offset + self.sps.frame_crop_right_offset) * crop_unit_x;
        }

        // Calculate height: only apply crop offsets if frame_cropping_flag is set
        let mut height = ((2 - self.sps.frame_mbs_only_flag as u32)
            * (self.sps.pic_height_in_map_units_minus1 + 1)
            * 16);
        if self.sps.frame_cropping_flag > 0 {
            // crop_unit_y = 2 for chroma_format_idc = 1 (4:2:0, default for Baseline)
            // For other chroma formats, this would need to be adjusted
            let crop_unit_y = if self.sps.chroma_format_idc == 0 {
                1
            } else {
                2
            };
            height -=
                (self.sps.frame_crop_top_offset + self.sps.frame_crop_bottom_offset) * crop_unit_y;
        }

        log::trace!("parsed sps data: {:?}", self.sps);
        Ok((width, height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    // ============================================
    // Test Fixtures - Real-world SPS NAL units
    // ============================================

    // SPS for Baseline profile, 640x480, no cropping
    // Profile: 66 (Baseline), Level: 30, Resolution: 640x480
    fn create_baseline_sps_640x480() -> BytesMut {
        let mut data = BytesMut::new();
        // profile_idc = 66 (Baseline)
        data.extend_from_slice(&[0x42]);
        // constraint flags = 0xE0, level_idc = 30
        data.extend_from_slice(&[0xE0, 0x1E]);
        // seq_parameter_set_id = 0 (ue(v): 1)
        data.extend_from_slice(&[0x80]);
        // log2_max_frame_num_minus4 = 0 (ue(v): 1)
        data.extend_from_slice(&[0x80]);
        // pic_order_cnt_type = 0 (ue(v): 1)
        data.extend_from_slice(&[0x80]);
        // log2_max_pic_order_cnt_lsb_minus4 = 0 (ue(v): 1)
        data.extend_from_slice(&[0x80]);
        // max_num_ref_frames = 1 (ue(v): 010)
        // gaps_in_frame_num_value_allowed_flag = 0 (1 bit, 4th bit of 0x40)
        // After reading 3 bits (010) from 0x40, we have 5 bits left: 00000
        // Read 1 bit for flag: 0 (4th bit), remaining: 0000
        // Align: discard 4 bits, next byte is 0x05
        data.extend_from_slice(&[0x40]);
        // pic_width_in_mbs_minus1 = 39 (640/16 - 1 = 39, ue(v): 00000101000)
        // Byte-aligned encoding: 0x05, 0x00 (uses 11 bits total)
        data.extend_from_slice(&[0x05, 0x00]);
        // pic_height_in_map_units_minus1 = 29 (480/16 - 1 = 29, ue(v): 000011110)
        // Uses 9 bits: first 8 bits from 0x0F (00001111), 9th bit (0) from next byte
        // frame_mbs_only_flag = 1 (2nd bit of next byte)
        // So we need: 0x0F (8 bits) + 0x40 (01000000: bit 0 = 9th bit of ue(v), bit 1 = frame_mbs_only_flag)
        data.extend_from_slice(&[0x0F, 0x40]);
        // direct_8x8_inference_flag = 1
        data.extend_from_slice(&[0x80]);
        // frame_cropping_flag = 0
        data.extend_from_slice(&[0x00]);
        // vui_parameters_present_flag = 0
        data.extend_from_slice(&[0x00]);
        data
    }

    // SPS for High profile, 1920x1080, with cropping
    // Profile: 100 (High), Level: 40, Resolution: 1920x1080
    fn create_high_sps_1920x1080() -> BytesMut {
        let mut data = BytesMut::new();
        // profile_idc = 100 (High)
        data.extend_from_slice(&[0x64]);
        // constraint flags = 0x00, level_idc = 40
        data.extend_from_slice(&[0x00, 0x28]);
        // seq_parameter_set_id = 0 (ue(v): 1)
        data.extend_from_slice(&[0x80]);
        // chroma_format_idc = 1 (ue(v): 010)
        data.extend_from_slice(&[0x40]);
        // bit_depth_luma_minus8 = 0 (ue(v): 1)
        data.extend_from_slice(&[0x80]);
        // bit_depth_chroma_minus8 = 0 (ue(v): 1)
        data.extend_from_slice(&[0x80]);
        // qpprime_y_zero_transform_bypass_flag = 0
        data.extend_from_slice(&[0x00]);
        // seq_scaling_matrix_present_flag = 0
        data.extend_from_slice(&[0x00]);
        // log2_max_frame_num_minus4 = 4 (ue(v): 00101)
        data.extend_from_slice(&[0x28]);
        // pic_order_cnt_type = 0 (ue(v): 1)
        data.extend_from_slice(&[0x80]);
        // log2_max_pic_order_cnt_lsb_minus4 = 4 (ue(v): 00101)
        data.extend_from_slice(&[0x28]);
        // max_num_ref_frames = 4 (ue(v): 00101)
        // gaps_in_frame_num_value_allowed_flag = 0 (1 bit, read from same byte after 00101)
        data.extend_from_slice(&[0x28]);
        // pic_width_in_mbs_minus1 = 119 (ue(v): 0000001111000, 13 bits)
        // Bytes: 0x03 (8 bits) + 0xC0 (5 bits used)
        data.extend_from_slice(&[0x03, 0xC0]);
        // pic_height_in_map_units_minus1 = 67 (ue(v): 0000001000100, 13 bits)
        // Bytes: 0x02 (8 bits) + 0x27 (bits 0-4: 00100 for pic_height, bits 5-7: 111 for flags)
        // Flags: frame_mbs_only_flag=1 (bit 5), direct_8x8_inference_flag=1 (bit 6), frame_cropping_flag=1 (bit 7)
        data.extend_from_slice(&[0x02, 0x27]);
        // frame_crop_left_offset = 0 (ue(v): 1)
        data.extend_from_slice(&[0x80]);
        // frame_crop_right_offset = 0 (ue(v): 1)
        data.extend_from_slice(&[0x80]);
        // frame_crop_top_offset = 0 (ue(v): 1)
        data.extend_from_slice(&[0x80]);
        // frame_crop_bottom_offset = 0 (ue(v): 1)
        data.extend_from_slice(&[0x80]);
        // vui_parameters_present_flag = 0
        data.extend_from_slice(&[0x00]);
        data
    }

    // ============================================
    // Basic Parsing Tests
    // ============================================

    #[test]
    fn test_sps_parser_new() {
        let bytes_reader = BytesReader::new(BytesMut::new());
        let parser = SpsParser::new(bytes_reader);
        assert_eq!(parser.sps.profile_idc, 0);
        assert_eq!(parser.sps.level_idc, 0);
    }

    #[test]
    fn test_sps_parser_extend_data() {
        let bytes_reader = BytesReader::new(BytesMut::new());
        let mut parser = SpsParser::new(bytes_reader);

        let mut data = BytesMut::new();
        data.extend_from_slice(&[0x42, 0xE0, 0x1E]);
        parser.extend_data(data);

        // Data should be extended to bits_reader
        assert!(parser.bits_reader.len() >= 24);
    }

    // ============================================
    // Profile IDC Tests
    // ============================================

    #[test]
    fn test_sps_parse_baseline_profile() {
        let mut data = BytesMut::new();
        // Minimal SPS for Baseline profile (66)
        data.extend_from_slice(&[0x42]); // profile_idc = 66
        data.extend_from_slice(&[0xE0, 0x1E]); // flags, level_idc = 30
        data.extend_from_slice(&[0x80]); // seq_parameter_set_id = 0
        data.extend_from_slice(&[0x80]); // log2_max_frame_num_minus4 = 0
        data.extend_from_slice(&[0x80]); // pic_order_cnt_type = 0
        data.extend_from_slice(&[0x80]); // log2_max_pic_order_cnt_lsb_minus4 = 0
        data.extend_from_slice(&[0x40]); // max_num_ref_frames = 1, gaps_in_frame_num_value_allowed_flag = 0 (4th bit)
        data.extend_from_slice(&[0x05, 0x00]); // pic_width_in_mbs_minus1 = 39 (640x480)
        // pic_height_in_map_units_minus1 = 29 (9 bits: 0x0F + 1 bit from next byte)
        // frame_mbs_only_flag = 1 (2nd bit of next byte)
        data.extend_from_slice(&[0x0F, 0x40]); // Combined: 0x0F (8 bits) + 0x40 (bit 0 = 9th bit of ue(v), bit 1 = frame_mbs_only_flag)
        data.extend_from_slice(&[0x80]); // direct_8x8_inference_flag = 1
        data.extend_from_slice(&[0x00]); // frame_cropping_flag = 0
        data.extend_from_slice(&[0x00]); // vui_parameters_present_flag = 0

        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        let (width, height) = result.unwrap();
        assert_eq!(parser.sps.profile_idc, 66);
        assert_eq!(parser.sps.level_idc, 30);
        // 640x480 resolution
        assert_eq!(width, 640);
        assert_eq!(height, 480);
    }

    #[test]
    fn test_sps_parse_main_profile() {
        let mut data = BytesMut::new();
        // Minimal SPS for Main profile (77)
        data.extend_from_slice(&[0x4D]); // profile_idc = 77
        data.extend_from_slice(&[0x00, 0x1E]); // flags, level_idc = 30
        data.extend_from_slice(&[0x80]); // seq_parameter_set_id = 0
        data.extend_from_slice(&[0x80]); // log2_max_frame_num_minus4 = 0
        data.extend_from_slice(&[0x80]); // pic_order_cnt_type = 0
        data.extend_from_slice(&[0x80]); // log2_max_pic_order_cnt_lsb_minus4 = 0
        // max_num_ref_frames = 1 (ue(v): 010, 3 bits) + gaps_in_frame_num_value_allowed_flag = 0 (1 bit)
        // 0x40 = 01000000: bits 0-2 = 010 (max_num_ref_frames), bit 3 = 0 (gaps flag)
        data.extend_from_slice(&[0x40]); // Combined: max_num_ref_frames + gaps flag
        data.extend_from_slice(&[0x05, 0x00]); // pic_width_in_mbs_minus1 = 39
        // pic_height_in_map_units_minus1 = 29 (9 bits: 0x0F + 1 bit from next byte)
        // frame_mbs_only_flag = 1 (2nd bit of next byte)
        // direct_8x8_inference_flag = 1 (3rd bit of next byte)
        // So: 0x0F (8 bits) + 0x60 (bits: 0=9th bit of ue(v), 1=frame_mbs_only_flag, 1=direct_8x8_inference_flag)
        data.extend_from_slice(&[0x0F, 0x60]);
        data.extend_from_slice(&[0x00]); // frame_cropping_flag = 0
        data.extend_from_slice(&[0x00]); // vui_parameters_present_flag = 0

        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.sps.profile_idc, 77);
    }

    #[test]
    fn test_sps_parse_high_profile() {
        let mut data = BytesMut::new();
        // Minimal SPS for High profile (100)
        data.extend_from_slice(&[0x64]); // profile_idc = 100
        data.extend_from_slice(&[0x00, 0x28]); // flags, level_idc = 40
        data.extend_from_slice(&[0x80]); // seq_parameter_set_id = 0
        data.extend_from_slice(&[0x40]); // chroma_format_idc = 1
        data.extend_from_slice(&[0x80]); // bit_depth_luma_minus8 = 0
        data.extend_from_slice(&[0x80]); // bit_depth_chroma_minus8 = 0
        data.extend_from_slice(&[0x00]); // qpprime_y_zero_transform_bypass_flag = 0
        data.extend_from_slice(&[0x00]); // seq_scaling_matrix_present_flag = 0
        data.extend_from_slice(&[0x28]); // log2_max_frame_num_minus4 = 4
        data.extend_from_slice(&[0x80]); // pic_order_cnt_type = 0
        data.extend_from_slice(&[0x28]); // log2_max_pic_order_cnt_lsb_minus4 = 4
        data.extend_from_slice(&[0x28]); // max_num_ref_frames = 4, gaps_in_frame_num_value_allowed_flag = 0 (6th bit)
        // pic_width_in_mbs_minus1 = 119 (ue(v): 0000001111000, 13 bits)
        data.extend_from_slice(&[0x03, 0xC0]);
        // pic_height_in_map_units_minus1 = 67 (ue(v): 0000001000100, 13 bits)
        // Bytes: 0x02 (8 bits) + 0x26 (bits 0-4: 00100 for pic_height, bits 5-7: 110 for flags)
        // Flags: frame_mbs_only_flag=1 (bit 5), direct_8x8_inference_flag=1 (bit 6), frame_cropping_flag=0 (bit 7)
        data.extend_from_slice(&[0x02, 0x26]);
        data.extend_from_slice(&[0x00]); // vui_parameters_present_flag = 0

        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.sps.profile_idc, 100);
        let (width, height) = result.unwrap();
        // 1920x1080 resolution
        assert_eq!(width, 1920);
        assert_eq!(height, 1080);
    }

    // ============================================
    // Resolution Extraction Tests
    // ============================================

    #[test]
    fn test_sps_resolution_640x480() {
        let data = create_baseline_sps_640x480();
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse().unwrap();

        assert_eq!(result.0, 640); // width
        assert_eq!(result.1, 480); // height
    }

    #[test]
    fn test_sps_resolution_1920x1080() {
        let data = create_high_sps_1920x1080();
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse().unwrap();

        assert_eq!(result.0, 1920); // width
        assert_eq!(result.1, 1080); // height
    }

    #[test]
    fn test_sps_resolution_with_cropping() {
        let mut data = BytesMut::new();
        // SPS with frame cropping
        data.extend_from_slice(&[0x42, 0xE0, 0x1E]); // profile, flags, level
        data.extend_from_slice(&[0x80]); // seq_parameter_set_id = 0
        data.extend_from_slice(&[0x80]); // log2_max_frame_num_minus4 = 0
        data.extend_from_slice(&[0x80]); // pic_order_cnt_type = 0
        data.extend_from_slice(&[0x80]); // log2_max_pic_order_cnt_lsb_minus4 = 0
        // max_num_ref_frames = 1 (ue(v): 010, 3 bits) + gaps_in_frame_num_value_allowed_flag = 0 (1 bit)
        // 0x40 = 01000000: bits 0-2 = 010 (max_num_ref_frames), bit 3 = 0 (gaps flag)
        data.extend_from_slice(&[0x40]); // Combined: max_num_ref_frames + gaps flag
        data.extend_from_slice(&[0x05, 0x00]); // pic_width_in_mbs_minus1 = 39
        // pic_height_in_map_units_minus1 = 29 (9 bits: 0x0F + 1 bit from next byte)
        // frame_mbs_only_flag = 1 (2nd bit of next byte)
        // direct_8x8_inference_flag = 1 (3rd bit of next byte)
        // frame_cropping_flag = 1 (4th bit of next byte)
        // So: 0x0F (8 bits) + 0x70 (bits: 0=9th bit of ue(v), 1=frame_mbs_only_flag, 1=direct_8x8_inference_flag, 1=frame_cropping_flag)
        data.extend_from_slice(&[0x0F, 0x70]);
        data.extend_from_slice(&[0x80]); // frame_crop_left_offset = 0
        data.extend_from_slice(&[0x80]); // frame_crop_right_offset = 0
        data.extend_from_slice(&[0x80]); // frame_crop_top_offset = 0
        data.extend_from_slice(&[0x80]); // frame_crop_bottom_offset = 0
        data.extend_from_slice(&[0x00]); // vui_parameters_present_flag = 0

        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse().unwrap();

        // With cropping offsets of 0, resolution should still be 640x480
        assert_eq!(result.0, 640);
        assert_eq!(result.1, 480);
        assert_eq!(parser.sps.frame_cropping_flag, 1);
    }

    // ============================================
    // Error Handling Tests
    // ============================================

    #[test]
    fn test_sps_parse_not_enough_data() {
        let mut data = BytesMut::new();
        data.extend_from_slice(&[0x42, 0xE0]); // Incomplete SPS
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_err());
    }

    #[test]
    fn test_sps_parse_empty_data() {
        let bytes_reader = BytesReader::new(BytesMut::new());
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_err());
    }

    // ============================================
    // Frame MBS Only Flag Tests
    // ============================================

    #[test]
    fn test_sps_frame_mbs_only_flag() {
        let mut data = BytesMut::new();
        // SPS with frame_mbs_only_flag = 0 (interlaced)
        data.extend_from_slice(&[0x42, 0xE0, 0x1E]); // profile, flags, level
        data.extend_from_slice(&[0x80]); // seq_parameter_set_id = 0
        data.extend_from_slice(&[0x80]); // log2_max_frame_num_minus4 = 0
        data.extend_from_slice(&[0x80]); // pic_order_cnt_type = 0
        data.extend_from_slice(&[0x80]); // log2_max_pic_order_cnt_lsb_minus4 = 0
        // max_num_ref_frames = 1 (ue(v): 010, 3 bits) + gaps_in_frame_num_value_allowed_flag = 0 (1 bit)
        // 0x40 = 01000000: bits 0-2 = 010 (max_num_ref_frames), bit 3 = 0 (gaps flag)
        data.extend_from_slice(&[0x40]); // Combined: max_num_ref_frames + gaps flag
        data.extend_from_slice(&[0x05, 0x00]); // pic_width_in_mbs_minus1 = 39
        // pic_height_in_map_units_minus1 = 29 (9 bits: 000011110)
        // After alignment, read from new byte: 0x0F (8 bits: 00001111) + 1 bit from next byte
        // frame_mbs_only_flag = 0, mb_adaptive_frame_field_flag = 0, direct_8x8_inference_flag = 1
        // So: 0x0F (8 bits) + 0x08 (bits: 0=9th bit of ue(v), 0=frame_mbs_only_flag, 0=mb_adaptive_frame_field_flag, 1=direct_8x8_inference_flag)
        data.extend_from_slice(&[0x0F, 0x08]);
        data.extend_from_slice(&[0x00]); // frame_cropping_flag = 0
        data.extend_from_slice(&[0x00]); // vui_parameters_present_flag = 0

        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse().unwrap();

        assert_eq!(parser.sps.frame_mbs_only_flag, 0);
        // Height should be doubled for interlaced (2 - 0) * 30 * 16 = 960
        assert_eq!(result.1, 960);
    }

    // ============================================
    // Pic Order Count Type Tests
    // ============================================

    #[test]
    fn test_sps_pic_order_cnt_type_0() {
        let mut data = BytesMut::new();
        // SPS with pic_order_cnt_type = 0
        data.extend_from_slice(&[0x42, 0xE0, 0x1E]);
        data.extend_from_slice(&[0x80]); // seq_parameter_set_id = 0
        data.extend_from_slice(&[0x80]); // log2_max_frame_num_minus4 = 0
        data.extend_from_slice(&[0x80]); // pic_order_cnt_type = 0
        data.extend_from_slice(&[0x80]); // log2_max_pic_order_cnt_lsb_minus4 = 0
        // max_num_ref_frames = 1 (ue(v): 010, 3 bits) + gaps_in_frame_num_value_allowed_flag = 0 (1 bit)
        // 0x40 = 01000000: bits 0-2 = 010 (max_num_ref_frames), bit 3 = 0 (gaps flag)
        data.extend_from_slice(&[0x40]); // Combined: max_num_ref_frames + gaps flag
        data.extend_from_slice(&[0x05, 0x00]); // pic_width_in_mbs_minus1 = 39
        // pic_height_in_map_units_minus1 = 29 (9 bits: 0x0F + 1 bit from next byte)
        // frame_mbs_only_flag = 1 (2nd bit of next byte)
        // direct_8x8_inference_flag = 1 (3rd bit of next byte)
        // So: 0x0F (8 bits) + 0x60 (bits: 0=9th bit of ue(v), 1=frame_mbs_only_flag, 1=direct_8x8_inference_flag)
        data.extend_from_slice(&[0x0F, 0x60]);
        data.extend_from_slice(&[0x00]); // frame_cropping_flag = 0
        data.extend_from_slice(&[0x00]); // vui_parameters_present_flag = 0

        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.sps.pic_order_cnt_type, 0);
    }

    #[test]
    fn test_sps_pic_order_cnt_type_1() {
        let mut data = BytesMut::new();
        // SPS with pic_order_cnt_type = 1
        data.extend_from_slice(&[0x42, 0xE0, 0x1E]);
        data.extend_from_slice(&[0x80]); // seq_parameter_set_id = 0
        data.extend_from_slice(&[0x80]); // log2_max_frame_num_minus4 = 0
        data.extend_from_slice(&[0x40]); // pic_order_cnt_type = 1 (ue(v): 010)
        data.extend_from_slice(&[0x00]); // delta_pic_order_always_zero_flag = 0
        data.extend_from_slice(&[0x80]); // offset_for_non_ref_pic = 0
        data.extend_from_slice(&[0x80]); // offset_for_top_to_bottom_field = 0
        data.extend_from_slice(&[0x80]); // num_ref_frames_in_pic_order_cnt_cycle = 0
        // max_num_ref_frames = 1 (ue(v): 010, 3 bits) + gaps_in_frame_num_value_allowed_flag = 0 (1 bit)
        // 0x40 = 01000000: bits 0-2 = 010 (max_num_ref_frames), bit 3 = 0 (gaps flag)
        data.extend_from_slice(&[0x40]); // Combined: max_num_ref_frames + gaps flag
        data.extend_from_slice(&[0x05, 0x00]); // pic_width_in_mbs_minus1 = 39
        // pic_height_in_map_units_minus1 = 29 (9 bits: 0x0F + 1 bit from next byte)
        // frame_mbs_only_flag = 1 (2nd bit of next byte)
        // direct_8x8_inference_flag = 1 (3rd bit of next byte)
        // So: 0x0F (8 bits) + 0x60 (bits: 0=9th bit of ue(v), 1=frame_mbs_only_flag, 1=direct_8x8_inference_flag)
        data.extend_from_slice(&[0x0F, 0x60]);
        data.extend_from_slice(&[0x00]); // frame_cropping_flag = 0
        data.extend_from_slice(&[0x00]); // vui_parameters_present_flag = 0

        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.sps.pic_order_cnt_type, 1);
    }
}
