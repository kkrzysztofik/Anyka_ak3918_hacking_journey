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
    pub bits_reader: BitsReader,
    pub sps: Sps,
}

impl SpsParser {
    pub fn new(reader: BytesReader) -> SpsParser {
        Self {
            bits_reader: BitsReader::new(reader),
            sps: Sps::default(),
        }
    }

    pub fn extend_data(&mut self, data: BytesMut) {
        self.bits_reader.extend_data(data);
    }

    pub fn parse(&mut self) -> Result<(u32, u32), H264Error> {
        self.sps.profile_idc = self.bits_reader.read_byte()?;
        tracing::debug!(profile_idc = self.sps.profile_idc, "parsed_profile_idc");
        self.sps.flag = self.bits_reader.read_byte()?;
        self.sps.level_idc = self.bits_reader.read_byte()?;
        tracing::debug!(level_idc = self.sps.level_idc, "parsed_level_idc");
        self.sps.seq_parameter_set_id = utils::read_uev(&mut self.bits_reader)?;

        self.parse_profile_specific_fields()?;

        self.sps.log2_max_frame_num_minus4 = utils::read_uev(&mut self.bits_reader)?;
        self.sps.pic_order_cnt_type = utils::read_uev(&mut self.bits_reader)?;

        self.parse_pic_order_cnt_fields()?;

        self.sps.max_num_ref_frames = utils::read_uev(&mut self.bits_reader)?;
        self.sps.gaps_in_frame_num_value_allowed_flag = self.bits_reader.read_bit()?;

        // H.264 spec: these fields are continuous bit-packed, no byte alignment
        self.sps.pic_width_in_mbs_minus1 = utils::read_uev(&mut self.bits_reader)?;
        self.sps.pic_height_in_map_units_minus1 = utils::read_uev(&mut self.bits_reader)?;

        self.sps.frame_mbs_only_flag = self.bits_reader.read_bit()?;

        if self.sps.frame_mbs_only_flag == 0 {
            self.sps.mb_adaptive_frame_field_flag = self.bits_reader.read_bit()?;
        }
        self.sps.direct_8x8_inference_flag = self.bits_reader.read_bit()?;
        self.sps.frame_cropping_flag = self.bits_reader.read_bit()?;

        self.parse_frame_cropping_fields()?;

        self.sps.vui_parameters_present_flag = self.bits_reader.read_bit()?;

        let width = self.calculate_width();
        let height = self.calculate_height();

        tracing::trace!(sps = ?self.sps, "parsed_sps_data");
        Ok((width, height))
    }

    /// Parse profile-specific fields (High profile and similar)
    fn parse_profile_specific_fields(&mut self) -> Result<(), H264Error> {
        match self.sps.profile_idc {
            100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 => self.parse_high_profile_fields(),
            _ => {
                // For profiles that don't include chroma_format_idc, default to 1 (4:2:0) per H.264 spec
                self.sps.chroma_format_idc = 1;
                Ok(())
            }
        }
    }

    /// Parse High profile specific fields
    fn parse_high_profile_fields(&mut self) -> Result<(), H264Error> {
        self.sps.chroma_format_idc = utils::read_uev(&mut self.bits_reader)?;
        if self.sps.chroma_format_idc == 3 {
            self.sps.separate_colour_plane_flag = self.bits_reader.read_bit()?;
        }
        self.sps.bit_depth_luma_minus8 = utils::read_uev(&mut self.bits_reader)?;
        self.sps.bit_depth_chroma_minus8 = utils::read_uev(&mut self.bits_reader)?;

        self.sps.qpprime_y_zero_transform_bypass_flag = self.bits_reader.read_bit()?;
        self.sps.seq_scaling_matrix_present_flag = self.bits_reader.read_bit()?;

        if self.sps.seq_scaling_matrix_present_flag > 0 {
            self.parse_scaling_matrix()?;
        }
        Ok(())
    }

    /// Parse scaling matrix when present
    fn parse_scaling_matrix(&mut self) -> Result<(), H264Error> {
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
        Ok(())
    }

    /// Parse picture order count fields based on type
    fn parse_pic_order_cnt_fields(&mut self) -> Result<(), H264Error> {
        match self.sps.pic_order_cnt_type {
            0 => {
                self.sps.log2_max_pic_order_cnt_lsb_minus4 =
                    utils::read_uev(&mut self.bits_reader)?;
            }
            1 => self.parse_pic_order_cnt_type1()?,
            _ => {}
        }
        Ok(())
    }

    /// Parse pic_order_cnt_type = 1 fields
    fn parse_pic_order_cnt_type1(&mut self) -> Result<(), H264Error> {
        self.sps.delta_pic_order_always_zero_flag = self.bits_reader.read_bit()?;
        self.sps.offset_for_non_ref_pic = utils::read_sev(&mut self.bits_reader)?;
        self.sps.offset_for_top_to_bottom_field = utils::read_sev(&mut self.bits_reader)?;
        self.sps.num_ref_frames_in_pic_order_cnt_cycle = utils::read_uev(&mut self.bits_reader)?;

        self.sps.offset_for_ref_frame.clear();
        if self.sps.num_ref_frames_in_pic_order_cnt_cycle > 0 {
            for _ in 0..self.sps.num_ref_frames_in_pic_order_cnt_cycle as usize {
                self.sps
                    .offset_for_ref_frame
                    .push(utils::read_sev(&mut self.bits_reader)?);
            }
        }
        Ok(())
    }

    /// Parse frame cropping fields if flag is set
    fn parse_frame_cropping_fields(&mut self) -> Result<(), H264Error> {
        if self.sps.frame_cropping_flag > 0 {
            self.sps.frame_crop_left_offset = utils::read_uev(&mut self.bits_reader)?;
            self.sps.frame_crop_right_offset = utils::read_uev(&mut self.bits_reader)?;
            self.sps.frame_crop_top_offset = utils::read_uev(&mut self.bits_reader)?;
            self.sps.frame_crop_bottom_offset = utils::read_uev(&mut self.bits_reader)?;
        } else {
            // Explicitly set to 0 when flag is not set
            self.sps.frame_crop_left_offset = 0;
            self.sps.frame_crop_right_offset = 0;
            self.sps.frame_crop_top_offset = 0;
            self.sps.frame_crop_bottom_offset = 0;
        }
        Ok(())
    }

    /// Calculate width with optional cropping
    fn calculate_width(&self) -> u32 {
        let mut width = (self.sps.pic_width_in_mbs_minus1 + 1) * 16;
        if self.sps.frame_cropping_flag > 0 {
            let crop_unit_x = self.get_crop_unit_x();
            width -=
                (self.sps.frame_crop_left_offset + self.sps.frame_crop_right_offset) * crop_unit_x;
        }
        width
    }

    /// Calculate height with optional cropping
    fn calculate_height(&self) -> u32 {
        let mut height = (2 - self.sps.frame_mbs_only_flag as u32)
            * (self.sps.pic_height_in_map_units_minus1 + 1)
            * 16;
        if self.sps.frame_cropping_flag > 0 {
            let crop_unit_y = self.get_crop_unit_y();
            height -=
                (self.sps.frame_crop_top_offset + self.sps.frame_crop_bottom_offset) * crop_unit_y;
        }
        height
    }

    /// Get crop unit X based on chroma format
    fn get_crop_unit_x(&self) -> u32 {
        match self.sps.chroma_format_idc {
            0 | 3 => 1,
            1 | 2 => 2,
            _ => 2, // Default to 2 for unknown formats
        }
    }

    /// Get crop unit Y based on chroma format
    fn get_crop_unit_y(&self) -> u32 {
        match self.sps.chroma_format_idc {
            0 | 3 => 1,
            1 => 2,
            2 => 1,
            _ => 2, // Default to 2 for unknown formats
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    // ============================================
    // SPS Encoder for Tests - Proper Bit-Packed Encoding
    // ============================================

    /// Helper to build proper bit-packed SPS NAL units for testing.
    /// Uses the BitsWriter to ensure continuous bit-packing per H.264 spec.
    struct SpsBuilder {
        writer: crate::bytesio::bits_writer::BitsWriter,
    }

    impl SpsBuilder {
        fn new() -> Self {
            Self {
                writer: crate::bytesio::bits_writer::BitsWriter::new(
                    crate::bytesio::bytes_writer::BytesWriter::default(),
                ),
            }
        }

        /// Write a u8 value (8 bits)
        fn write_u8(&mut self, val: u8) {
            for i in (0..8).rev() {
                self.writer.write_bit((val >> i) & 1).unwrap();
            }
        }

        /// Write n bits from a u32 value (most significant bit first)
        fn write_bits(&mut self, val: u32, n: usize) {
            for i in (0..n).rev() {
                self.writer.write_bit(((val >> i) & 1) as u8).unwrap();
            }
        }

        /// Write a single bit
        fn write_bit(&mut self, val: u8) {
            self.writer.write_bit(val).unwrap();
        }

        /// Write unsigned Exp-Golomb coded value (ue(v))
        /// Encoding: value N = 2^leadingZeroBits - 1 + INFO
        /// Written as: leadingZeroBits zeros, 1 bit, leadingZeroBits INFO bits
        fn write_uev(&mut self, val: u32) {
            if val == 0 {
                // Special case: 0 is encoded as just "1"
                self.write_bit(1);
                return;
            }

            // Find leadingZeroBits such that 2^leadingZeroBits - 1 <= val < 2^(leadingZeroBits+1) - 1
            // leadingZeroBits = floor(log2(val + 1))
            let code_num = val + 1;
            let leading_zero_bits = 31 - code_num.leading_zeros() as usize; // This is floor(log2(code_num))
            let info = val - ((1 << leading_zero_bits) - 1);

            // Write leading zeros
            for _ in 0..leading_zero_bits {
                self.write_bit(0);
            }
            // Write 1
            self.write_bit(1);
            // Write info bits
            self.write_bits(info, leading_zero_bits);
        }

        /// Write signed Exp-Golomb coded value (se(v))
        fn write_sev(&mut self, val: i32) {
            let code_num = if val <= 0 {
                (-2 * val) as u32
            } else {
                (2 * val - 1) as u32
            };
            self.write_uev(code_num);
        }

        /// Get the encoded bytes, flushing any remaining bits
        fn build(mut self) -> BytesMut {
            // Flush any partial byte
            self.writer.bits_alignment_8().unwrap();
            self.writer.get_current_bytes()
        }
    }

    /// Create a properly bit-packed Baseline SPS for given resolution
    fn create_baseline_sps(width: u32, height: u32) -> BytesMut {
        let mut builder = SpsBuilder::new();

        // First 3 bytes are byte-aligned
        builder.write_u8(0x42); // profile_idc = 66 (Baseline)
        builder.write_u8(0xE0); // constraint flags
        builder.write_u8(0x1E); // level_idc = 30

        // Now continuous bit-packed fields
        builder.write_uev(0); // seq_parameter_set_id = 0
        builder.write_uev(0); // log2_max_frame_num_minus4 = 0
        builder.write_uev(2); // pic_order_cnt_type = 2 (no extra fields needed)
        builder.write_uev(1); // max_num_ref_frames = 1
        builder.write_bit(0); // gaps_in_frame_num_value_allowed_flag = 0
        builder.write_uev(width / 16 - 1); // pic_width_in_mbs_minus1
        builder.write_uev(height / 16 - 1); // pic_height_in_map_units_minus1
        builder.write_bit(1); // frame_mbs_only_flag = 1
        builder.write_bit(1); // direct_8x8_inference_flag = 1
        builder.write_bit(0); // frame_cropping_flag = 0
        builder.write_bit(0); // vui_parameters_present_flag = 0

        builder.build()
    }

    /// Create a properly bit-packed Baseline SPS with pic_order_cnt_type = 0
    fn create_baseline_sps_poc0(width: u32, height: u32) -> BytesMut {
        let mut builder = SpsBuilder::new();

        builder.write_u8(0x42); // profile_idc = 66 (Baseline)
        builder.write_u8(0xE0); // constraint flags
        builder.write_u8(0x1E); // level_idc = 30

        builder.write_uev(0); // seq_parameter_set_id = 0
        builder.write_uev(0); // log2_max_frame_num_minus4 = 0
        builder.write_uev(0); // pic_order_cnt_type = 0
        builder.write_uev(0); // log2_max_pic_order_cnt_lsb_minus4 = 0
        builder.write_uev(1); // max_num_ref_frames = 1
        builder.write_bit(0); // gaps_in_frame_num_value_allowed_flag = 0
        builder.write_uev(width / 16 - 1); // pic_width_in_mbs_minus1
        builder.write_uev(height / 16 - 1); // pic_height_in_map_units_minus1
        builder.write_bit(1); // frame_mbs_only_flag = 1
        builder.write_bit(1); // direct_8x8_inference_flag = 1
        builder.write_bit(0); // frame_cropping_flag = 0
        builder.write_bit(0); // vui_parameters_present_flag = 0

        builder.build()
    }

    /// Create a properly bit-packed Baseline SPS with pic_order_cnt_type = 1
    fn create_baseline_sps_poc1(width: u32, height: u32) -> BytesMut {
        let mut builder = SpsBuilder::new();

        builder.write_u8(0x42); // profile_idc = 66 (Baseline)
        builder.write_u8(0xE0); // constraint flags
        builder.write_u8(0x1E); // level_idc = 30

        builder.write_uev(0); // seq_parameter_set_id = 0
        builder.write_uev(0); // log2_max_frame_num_minus4 = 0
        builder.write_uev(1); // pic_order_cnt_type = 1
        builder.write_bit(0); // delta_pic_order_always_zero_flag = 0
        builder.write_uev(0); // offset_for_non_ref_pic = 0 (se(v) where 0 = ue(0) = 1)
        builder.write_uev(0); // offset_for_top_to_bottom_field = 0
        builder.write_uev(0); // num_ref_frames_in_pic_order_cnt_cycle = 0
        builder.write_uev(1); // max_num_ref_frames = 1
        builder.write_bit(0); // gaps_in_frame_num_value_allowed_flag = 0
        builder.write_uev(width / 16 - 1); // pic_width_in_mbs_minus1
        builder.write_uev(height / 16 - 1); // pic_height_in_map_units_minus1
        builder.write_bit(1); // frame_mbs_only_flag = 1
        builder.write_bit(1); // direct_8x8_inference_flag = 1
        builder.write_bit(0); // frame_cropping_flag = 0
        builder.write_bit(0); // vui_parameters_present_flag = 0

        builder.build()
    }

    /// Create a properly bit-packed Baseline SPS with frame_mbs_only_flag = 0 (interlaced)
    fn create_baseline_sps_interlaced(width: u32, height: u32) -> BytesMut {
        let mut builder = SpsBuilder::new();

        builder.write_u8(0x42); // profile_idc = 66 (Baseline)
        builder.write_u8(0xE0); // constraint flags
        builder.write_u8(0x1E); // level_idc = 30

        builder.write_uev(0); // seq_parameter_set_id = 0
        builder.write_uev(0); // log2_max_frame_num_minus4 = 0
        builder.write_uev(2); // pic_order_cnt_type = 2
        builder.write_uev(1); // max_num_ref_frames = 1
        builder.write_bit(0); // gaps_in_frame_num_value_allowed_flag = 0
        builder.write_uev(width / 16 - 1); // pic_width_in_mbs_minus1
                                           // For interlaced, height is halved in map units
        builder.write_uev(height / 16 / 2 - 1); // pic_height_in_map_units_minus1
        builder.write_bit(0); // frame_mbs_only_flag = 0 (interlaced)
        builder.write_bit(0); // mb_adaptive_frame_field_flag = 0
        builder.write_bit(1); // direct_8x8_inference_flag = 1
        builder.write_bit(0); // frame_cropping_flag = 0
        builder.write_bit(0); // vui_parameters_present_flag = 0

        builder.build()
    }

    /// Create a properly bit-packed Baseline SPS with frame cropping
    fn create_baseline_sps_with_cropping(
        width: u32,
        height: u32,
        crop_left: u32,
        crop_right: u32,
        crop_top: u32,
        crop_bottom: u32,
    ) -> BytesMut {
        let mut builder = SpsBuilder::new();

        builder.write_u8(0x42); // profile_idc = 66 (Baseline)
        builder.write_u8(0xE0); // constraint flags
        builder.write_u8(0x1E); // level_idc = 30

        builder.write_uev(0); // seq_parameter_set_id = 0
        builder.write_uev(0); // log2_max_frame_num_minus4 = 0
        builder.write_uev(2); // pic_order_cnt_type = 2
        builder.write_uev(1); // max_num_ref_frames = 1
        builder.write_bit(0); // gaps_in_frame_num_value_allowed_flag = 0
        builder.write_uev(width / 16 - 1); // pic_width_in_mbs_minus1
        builder.write_uev(height / 16 - 1); // pic_height_in_map_units_minus1
        builder.write_bit(1); // frame_mbs_only_flag = 1
        builder.write_bit(1); // direct_8x8_inference_flag = 1
        builder.write_bit(1); // frame_cropping_flag = 1
        builder.write_uev(crop_left); // frame_crop_left_offset
        builder.write_uev(crop_right); // frame_crop_right_offset
        builder.write_uev(crop_top); // frame_crop_top_offset
        builder.write_uev(crop_bottom); // frame_crop_bottom_offset
        builder.write_bit(0); // vui_parameters_present_flag = 0

        builder.build()
    }

    /// Create a properly bit-packed High profile SPS
    fn create_high_sps(width: u32, height: u32) -> BytesMut {
        let mut builder = SpsBuilder::new();

        builder.write_u8(0x64); // profile_idc = 100 (High)
        builder.write_u8(0x00); // constraint flags
        builder.write_u8(0x28); // level_idc = 40

        builder.write_uev(0); // seq_parameter_set_id = 0
        builder.write_uev(1); // chroma_format_idc = 1 (4:2:0)
        builder.write_uev(0); // bit_depth_luma_minus8 = 0
        builder.write_uev(0); // bit_depth_chroma_minus8 = 0
        builder.write_bit(0); // qpprime_y_zero_transform_bypass_flag = 0
        builder.write_bit(0); // seq_scaling_matrix_present_flag = 0
        builder.write_uev(0); // log2_max_frame_num_minus4 = 0
        builder.write_uev(2); // pic_order_cnt_type = 2 (simpler, no extra fields)
        builder.write_uev(2); // max_num_ref_frames = 2
        builder.write_bit(0); // gaps_in_frame_num_value_allowed_flag = 0
        builder.write_uev(width / 16 - 1); // pic_width_in_mbs_minus1
        builder.write_uev(height / 16 - 1); // pic_height_in_map_units_minus1
        builder.write_bit(1); // frame_mbs_only_flag = 1
        builder.write_bit(1); // direct_8x8_inference_flag = 1
        builder.write_bit(0); // frame_cropping_flag = 0
        builder.write_bit(0); // vui_parameters_present_flag = 0

        builder.build()
    }

    /// Create a High profile SPS with chroma_format_idc = 3 and scaling matrix present
    fn create_high_sps_chroma3_with_scaling(width: u32, height: u32) -> BytesMut {
        let mut builder = SpsBuilder::new();

        builder.write_u8(0x64); // profile_idc = 100 (High)
        builder.write_u8(0x00); // constraint flags
        builder.write_u8(0x28); // level_idc = 40

        builder.write_uev(0); // seq_parameter_set_id = 0
        builder.write_uev(3); // chroma_format_idc = 3
        builder.write_bit(1); // separate_colour_plane_flag = 1
        builder.write_uev(0); // bit_depth_luma_minus8 = 0
        builder.write_uev(0); // bit_depth_chroma_minus8 = 0
        builder.write_bit(0); // qpprime_y_zero_transform_bypass_flag = 0
        builder.write_bit(1); // seq_scaling_matrix_present_flag = 1

        // seq_scaling_list_present_flag[0..7]
        builder.write_bit(1);
        builder.write_bit(0);
        builder.write_bit(1);
        builder.write_bit(0);
        builder.write_bit(1);
        builder.write_bit(0);
        builder.write_bit(1);
        builder.write_bit(0);

        builder.write_uev(0); // log2_max_frame_num_minus4 = 0
        builder.write_uev(2); // pic_order_cnt_type = 2
        builder.write_uev(1); // max_num_ref_frames = 1
        builder.write_bit(0); // gaps_in_frame_num_value_allowed_flag = 0
        builder.write_uev(width / 16 - 1); // pic_width_in_mbs_minus1
        builder.write_uev(height / 16 - 1); // pic_height_in_map_units_minus1
        builder.write_bit(1); // frame_mbs_only_flag = 1
        builder.write_bit(1); // direct_8x8_inference_flag = 1
        builder.write_bit(0); // frame_cropping_flag = 0
        builder.write_bit(0); // vui_parameters_present_flag = 0

        builder.build()
    }

    /// Create a Baseline SPS with pic_order_cnt_type = 1 and non-zero offsets
    fn create_baseline_sps_poc1_with_offsets(width: u32, height: u32) -> BytesMut {
        let mut builder = SpsBuilder::new();

        builder.write_u8(0x42); // profile_idc = 66 (Baseline)
        builder.write_u8(0xE0); // constraint flags
        builder.write_u8(0x1E); // level_idc = 30

        builder.write_uev(0); // seq_parameter_set_id = 0
        builder.write_uev(0); // log2_max_frame_num_minus4 = 0
        builder.write_uev(1); // pic_order_cnt_type = 1
        builder.write_bit(0); // delta_pic_order_always_zero_flag = 0
        builder.write_sev(-1); // offset_for_non_ref_pic
        builder.write_sev(2); // offset_for_top_to_bottom_field
        builder.write_uev(2); // num_ref_frames_in_pic_order_cnt_cycle = 2
        builder.write_sev(-2); // offset_for_ref_frame[0]
        builder.write_sev(3); // offset_for_ref_frame[1]
        builder.write_uev(1); // max_num_ref_frames = 1
        builder.write_bit(0); // gaps_in_frame_num_value_allowed_flag = 0
        builder.write_uev(width / 16 - 1); // pic_width_in_mbs_minus1
        builder.write_uev(height / 16 - 1); // pic_height_in_map_units_minus1
        builder.write_bit(1); // frame_mbs_only_flag = 1
        builder.write_bit(1); // direct_8x8_inference_flag = 1
        builder.write_bit(0); // frame_cropping_flag = 0
        builder.write_bit(0); // vui_parameters_present_flag = 0

        builder.build()
    }

    /// Create a properly bit-packed Main profile SPS
    fn create_main_sps(width: u32, height: u32) -> BytesMut {
        let mut builder = SpsBuilder::new();

        builder.write_u8(0x4D); // profile_idc = 77 (Main)
        builder.write_u8(0x00); // constraint flags
        builder.write_u8(0x1E); // level_idc = 30

        builder.write_uev(0); // seq_parameter_set_id = 0
        builder.write_uev(0); // log2_max_frame_num_minus4 = 0
        builder.write_uev(2); // pic_order_cnt_type = 2
        builder.write_uev(1); // max_num_ref_frames = 1
        builder.write_bit(0); // gaps_in_frame_num_value_allowed_flag = 0
        builder.write_uev(width / 16 - 1); // pic_width_in_mbs_minus1
        builder.write_uev(height / 16 - 1); // pic_height_in_map_units_minus1
        builder.write_bit(1); // frame_mbs_only_flag = 1
        builder.write_bit(1); // direct_8x8_inference_flag = 1
        builder.write_bit(0); // frame_cropping_flag = 0
        builder.write_bit(0); // vui_parameters_present_flag = 0

        builder.build()
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
        let data = create_baseline_sps(640, 480);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        let (width, height) = result.unwrap();
        assert_eq!(parser.sps.profile_idc, 66);
        assert_eq!(parser.sps.level_idc, 30);
        assert_eq!(width, 640);
        assert_eq!(height, 480);
    }

    #[test]
    fn test_sps_parse_main_profile() {
        let data = create_main_sps(640, 480);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.sps.profile_idc, 77);
    }

    #[test]
    fn test_sps_parse_high_profile() {
        let data = create_high_sps(1920, 1088); // Use 1088 (divisible by 16)
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.sps.profile_idc, 100);
        let (width, height) = result.unwrap();
        assert_eq!(width, 1920);
        assert_eq!(height, 1088);
    }

    #[test]
    fn test_sps_parse_high_profile_chroma3_with_scaling_matrix() {
        let data = create_high_sps_chroma3_with_scaling(1280, 720);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);

        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.sps.profile_idc, 100);
        assert_eq!(parser.sps.chroma_format_idc, 3);
        assert_eq!(parser.sps.separate_colour_plane_flag, 1);
        assert_eq!(parser.sps.seq_scaling_matrix_present_flag, 1);
        assert_eq!(parser.sps.seq_scaling_list_present_flag.len(), 8);
        assert_eq!(
            parser.sps.seq_scaling_list_present_flag,
            vec![1, 0, 1, 0, 1, 0, 1, 0]
        );
    }

    // ============================================
    // Resolution Extraction Tests
    // ============================================

    #[test]
    fn test_sps_resolution_640x480() {
        let data = create_baseline_sps(640, 480);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse().unwrap();

        assert_eq!(result.0, 640); // width
        assert_eq!(result.1, 480); // height
    }

    #[test]
    fn test_sps_resolution_1920x1080() {
        // For 1920x1080, we need cropping since 1080 is not divisible by 16
        // Use 1920x1088 raw, then crop 8 pixels (4 units) from bottom
        // crop_unit_y = 2 for 4:2:0, so crop_bottom = 4 units removes 8 pixels
        let data = create_baseline_sps_with_cropping(1920, 1088, 0, 0, 0, 4);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse().unwrap();

        assert_eq!(result.0, 1920);
        assert_eq!(result.1, 1080); // 1088 - 4*2 = 1080
    }

    #[test]
    fn test_sps_resolution_with_cropping() {
        // Zero cropping
        let data = create_baseline_sps_with_cropping(640, 480, 0, 0, 0, 0);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse().unwrap();

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
        let data = create_baseline_sps_interlaced(640, 480);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse().unwrap();

        assert_eq!(parser.sps.frame_mbs_only_flag, 0);
        // Height should be doubled for interlaced: (2 - 0) * (480/16/2) * 16 = 480
        // With interlaced, pic_height_in_map_units_minus1 = 14 (480/16/2 - 1)
        // height = (2 - 0) * (14 + 1) * 16 = 2 * 15 * 16 = 480
        assert_eq!(result.1, 480);
    }

    // ============================================
    // Pic Order Count Type Tests
    // ============================================

    #[test]
    fn test_sps_pic_order_cnt_type_0() {
        let data = create_baseline_sps_poc0(640, 480);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.sps.pic_order_cnt_type, 0);
    }

    #[test]
    fn test_sps_pic_order_cnt_type_1() {
        let data = create_baseline_sps_poc1(640, 480);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.sps.pic_order_cnt_type, 1);
    }

    #[test]
    fn test_sps_pic_order_cnt_type_1_with_offsets() {
        let data = create_baseline_sps_poc1_with_offsets(640, 480);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);

        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.sps.pic_order_cnt_type, 1);
        assert_eq!(parser.sps.delta_pic_order_always_zero_flag, 0);
        assert_eq!(parser.sps.offset_for_non_ref_pic, -1);
        assert_eq!(parser.sps.offset_for_top_to_bottom_field, 2);
        assert_eq!(parser.sps.num_ref_frames_in_pic_order_cnt_cycle, 2);
        assert_eq!(parser.sps.offset_for_ref_frame, vec![-2, 3]);
    }

    // ============================================
    // Sps Default Tests
    // ============================================

    #[test]
    fn test_sps_default() {
        let sps = Sps::default();
        assert_eq!(sps.profile_idc, 0);
        assert_eq!(sps.level_idc, 0);
        assert_eq!(sps.chroma_format_idc, 0);
        assert_eq!(sps.frame_mbs_only_flag, 0);
        assert_eq!(sps.frame_cropping_flag, 0);
        assert_eq!(sps.vui_parameters_present_flag, 0);
        assert!(sps.seq_scaling_list_present_flag.is_empty());
        assert!(sps.offset_for_ref_frame.is_empty());
    }

    // ============================================
    // Resolution Variety Tests
    // ============================================

    #[test]
    fn test_sps_resolution_320x240() {
        let data = create_baseline_sps(320, 240);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let (w, h) = parser.parse().unwrap();
        assert_eq!(w, 320);
        assert_eq!(h, 240);
    }

    #[test]
    fn test_sps_resolution_1280x720() {
        let data = create_baseline_sps(1280, 720);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let (w, h) = parser.parse().unwrap();
        assert_eq!(w, 1280);
        assert_eq!(h, 720);
    }

    #[test]
    fn test_sps_resolution_1920x1088_high_profile() {
        let data = create_high_sps(1920, 1088);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let (w, h) = parser.parse().unwrap();
        assert_eq!(w, 1920);
        assert_eq!(h, 1088);
    }

    // ============================================
    // Cropping Unit Tests
    // ============================================

    #[test]
    fn test_sps_cropping_left_and_right() {
        // Crop 16 pixels from left (8 units) and 16 from right (8 units)
        // For 4:2:0 (chroma_format_idc=1): crop_unit_x = 2
        // Width: 640 - (8+8)*2 = 640 - 32 = 608
        let data = create_baseline_sps_with_cropping(640, 480, 8, 8, 0, 0);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let (w, h) = parser.parse().unwrap();
        assert_eq!(w, 608);
        assert_eq!(h, 480);
    }

    #[test]
    fn test_sps_cropping_top_and_bottom() {
        // For 4:2:0 (chroma_format_idc=1): crop_unit_y = 2
        // Height: 480 - (0+4)*2 = 480 - 8 = 472
        let data = create_baseline_sps_with_cropping(640, 480, 0, 0, 0, 4);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        let (w, h) = parser.parse().unwrap();
        assert_eq!(w, 640);
        assert_eq!(h, 472);
    }

    // ============================================
    // Profile IDC Variety Tests
    // ============================================

    #[test]
    fn test_sps_chroma_format_idc_defaults_for_baseline() {
        // Baseline (profile_idc=66) should default chroma_format_idc to 1
        let data = create_baseline_sps(640, 480);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        parser.parse().unwrap();
        assert_eq!(parser.sps.chroma_format_idc, 1);
    }

    #[test]
    fn test_sps_high_profile_chroma_format_parsed() {
        let data = create_high_sps(640, 480);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        parser.parse().unwrap();
        assert_eq!(parser.sps.profile_idc, 100);
        assert_eq!(parser.sps.chroma_format_idc, 1);
    }

    #[test]
    fn test_sps_pic_order_cnt_type_2() {
        // Type 2 requires no extra fields
        let data = create_baseline_sps(640, 480);
        let bytes_reader = BytesReader::new(data);
        let mut parser = SpsParser::new(bytes_reader);
        parser.parse().unwrap();
        assert_eq!(parser.sps.pic_order_cnt_type, 2);
    }

    #[test]
    fn test_sps_debug_format() {
        let sps = Sps::default();
        let debug = format!("{sps:?}");
        assert!(debug.contains("Sps"));
        assert!(debug.contains("profile_idc"));
    }
}
