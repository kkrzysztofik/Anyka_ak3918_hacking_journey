use {
    super::errors::H264Error, super::utils, crate::bytesio::bits_reader::BitsReader,
    crate::bytesio::bytes_reader::BytesReader, bytes::BytesMut,
};

#[derive(Default, Debug)]
pub struct Pps {
    pub pic_parameter_set_id: u32,                        // ue(v)
    pub seq_parameter_set_id: u32,                        // ue(v)
    pub entropy_coding_mode_flag: u8,                     // u(1)
    pub bottom_field_pic_order_in_frame_present_flag: u8, // u(1)
    pub num_slice_groups_minus1: u32,                     // ue(v)
    // slice_group_info fields
    pub slice_group_map_type: u32, // ue(v) - when num_slice_groups_minus1 > 0
    pub slice_group_info: Vec<u32>, // Additional slice group information
}

pub struct PpsParser {
    pub bytes_reader: BytesReader,
    pub bits_reader: BitsReader,
    pub pps: Pps,
}

impl PpsParser {
    pub fn new(reader: BytesReader) -> PpsParser {
        Self {
            bytes_reader: BytesReader::new(BytesMut::new()),
            bits_reader: BitsReader::new(reader),
            pps: Pps::default(),
        }
    }

    pub fn extend_data(&mut self, data: BytesMut) {
        self.bits_reader.extend_data(data);
    }

    pub fn parse(&mut self) -> Result<(), H264Error> {
        self.pps.pic_parameter_set_id = utils::read_uev(&mut self.bits_reader)?;
        self.pps.seq_parameter_set_id = utils::read_uev(&mut self.bits_reader)?;
        self.pps.entropy_coding_mode_flag = self.bits_reader.read_bit()?;
        self.pps.bottom_field_pic_order_in_frame_present_flag = self.bits_reader.read_bit()?;
        self.pps.num_slice_groups_minus1 = utils::read_uev(&mut self.bits_reader)?;

        // Parse slice_group_info if num_slice_groups_minus1 > 0
        if self.pps.num_slice_groups_minus1 > 0 {
            self.pps.slice_group_map_type = utils::read_uev(&mut self.bits_reader)?;

            match self.pps.slice_group_map_type {
                0 => {
                    // Interleaved slice group map type
                    for _ in 0..=self.pps.num_slice_groups_minus1 {
                        let run_length_minus1 = utils::read_uev(&mut self.bits_reader)?;
                        self.pps.slice_group_info.push(run_length_minus1);
                    }
                }
                2 => {
                    // Foreground with left-over slice group map type
                    for _ in 0..self.pps.num_slice_groups_minus1 {
                        let top_left = utils::read_uev(&mut self.bits_reader)?;
                        let bottom_right = utils::read_uev(&mut self.bits_reader)?;
                        self.pps.slice_group_info.push(top_left);
                        self.pps.slice_group_info.push(bottom_right);
                    }
                }
                3..=5 => {
                    // Slice group map units
                    let num_slice_group_map_units_minus1 = utils::read_uev(&mut self.bits_reader)?;
                    self.pps
                        .slice_group_info
                        .push(num_slice_group_map_units_minus1);
                    // Additional parsing would be needed for full implementation
                }
                6 => {
                    // Explicit slice group map
                    let num_slice_group_map_units_minus1 = utils::read_uev(&mut self.bits_reader)?;
                    self.pps
                        .slice_group_info
                        .push(num_slice_group_map_units_minus1);
                    for _ in 0..=num_slice_group_map_units_minus1 {
                        let slice_group_id = utils::read_uev(&mut self.bits_reader)?;
                        self.pps.slice_group_info.push(slice_group_id);
                    }
                }
                _ => {
                    // Types 1, 7 are not used or reserved
                }
            }
        }

        log::trace!("parsed pps data: {:?}", self.pps);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    // ============================================
    // PPS Encoder for Tests - Proper Bit-Packed Encoding
    // ============================================

    /// Helper to build proper bit-packed PPS NAL units for testing.
    /// Uses the BitsWriter to ensure continuous bit-packing per H.264 spec.
    struct PpsBuilder {
        writer: crate::bytesio::bits_writer::BitsWriter,
    }

    impl PpsBuilder {
        fn new() -> Self {
            Self {
                writer: crate::bytesio::bits_writer::BitsWriter::new(
                    crate::bytesio::bytes_writer::BytesWriter::default(),
                ),
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
        fn write_uev(&mut self, val: u32) {
            if val == 0 {
                self.write_bit(1);
                return;
            }

            let code_num = val + 1;
            let leading_zero_bits = 31 - code_num.leading_zeros() as usize;
            let info = val - ((1 << leading_zero_bits) - 1);

            for _ in 0..leading_zero_bits {
                self.write_bit(0);
            }
            self.write_bit(1);
            self.write_bits(info, leading_zero_bits);
        }

        /// Get the encoded bytes, flushing any remaining bits
        fn build(mut self) -> BytesMut {
            self.writer.bits_aligment_8().unwrap();
            self.writer.get_current_bytes()
        }
    }

    // ============================================
    // Test Fixtures - Properly bit-packed PPS NAL units
    // ============================================

    /// PPS for Baseline profile with typical settings
    /// pic_parameter_set_id = 0, seq_parameter_set_id = 0
    /// entropy_coding_mode_flag = 0 (CAVLC), num_slice_groups_minus1 = 0
    fn create_baseline_pps() -> BytesMut {
        let mut builder = PpsBuilder::new();

        builder.write_uev(0); // pic_parameter_set_id = 0
        builder.write_uev(0); // seq_parameter_set_id = 0
        builder.write_bit(0); // entropy_coding_mode_flag = 0 (CAVLC)
        builder.write_bit(0); // bottom_field_pic_order_in_frame_present_flag = 0
        builder.write_uev(0); // num_slice_groups_minus1 = 0

        builder.build()
    }

    /// PPS for High profile with CABAC (entropy_coding_mode_flag = 1)
    /// pic_parameter_set_id = 1, seq_parameter_set_id = 0
    fn create_high_pps_cabac() -> BytesMut {
        let mut builder = PpsBuilder::new();

        builder.write_uev(1); // pic_parameter_set_id = 1
        builder.write_uev(0); // seq_parameter_set_id = 0
        builder.write_bit(1); // entropy_coding_mode_flag = 1 (CABAC)
        builder.write_bit(0); // bottom_field_pic_order_in_frame_present_flag = 0
        builder.write_uev(0); // num_slice_groups_minus1 = 0

        builder.build()
    }

    /// PPS with slice groups (num_slice_groups_minus1 > 0)
    /// pic_parameter_set_id = 2, slice_group_map_type = 0 (interleaved)
    fn create_pps_with_slice_groups() -> BytesMut {
        let mut builder = PpsBuilder::new();

        builder.write_uev(2); // pic_parameter_set_id = 2
        builder.write_uev(0); // seq_parameter_set_id = 0
        builder.write_bit(0); // entropy_coding_mode_flag = 0
        builder.write_bit(0); // bottom_field_pic_order_in_frame_present_flag = 0
        builder.write_uev(1); // num_slice_groups_minus1 = 1 (2 slice groups)
        builder.write_uev(0); // slice_group_map_type = 0 (interleaved)
        builder.write_uev(10); // run_length_minus1[0] = 10
        builder.write_uev(20); // run_length_minus1[1] = 20

        builder.build()
    }

    // ============================================
    // Basic Parsing Tests
    // ============================================

    #[test]
    fn test_pps_parser_new() {
        let bytes_reader = BytesReader::new(BytesMut::new());
        let parser = PpsParser::new(bytes_reader);
        assert_eq!(parser.pps.pic_parameter_set_id, 0);
        assert_eq!(parser.pps.seq_parameter_set_id, 0);
        assert_eq!(parser.pps.entropy_coding_mode_flag, 0);
    }

    #[test]
    fn test_pps_parser_extend_data() {
        let bytes_reader = BytesReader::new(BytesMut::new());
        let mut parser = PpsParser::new(bytes_reader);

        let mut data = BytesMut::new();
        data.extend_from_slice(&[0x80, 0x80]);
        parser.extend_data(data);

        // Data should be extended to bits_reader
        assert!(parser.bits_reader.len() >= 16);
    }

    // ============================================
    // Field Validation Tests
    // ============================================

    #[test]
    fn test_pps_parse_pic_parameter_set_id() {
        let data = create_baseline_pps();
        let bytes_reader = BytesReader::new(data);
        let mut parser = PpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.pps.pic_parameter_set_id, 0);
    }

    #[test]
    fn test_pps_parse_seq_parameter_set_id() {
        let data = create_baseline_pps();
        let bytes_reader = BytesReader::new(data);
        let mut parser = PpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.pps.seq_parameter_set_id, 0);
    }

    #[test]
    fn test_pps_parse_entropy_coding_mode_flag_cavlc() {
        let data = create_baseline_pps();
        let bytes_reader = BytesReader::new(data);
        let mut parser = PpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.pps.entropy_coding_mode_flag, 0); // CAVLC
    }

    #[test]
    fn test_pps_parse_entropy_coding_mode_flag_cabac() {
        let data = create_high_pps_cabac();
        let bytes_reader = BytesReader::new(data);
        let mut parser = PpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.pps.entropy_coding_mode_flag, 1); // CABAC
        assert_eq!(parser.pps.pic_parameter_set_id, 1);
    }

    #[test]
    fn test_pps_parse_slice_group_info() {
        let data = create_pps_with_slice_groups();
        let bytes_reader = BytesReader::new(data);
        let mut parser = PpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.pps.num_slice_groups_minus1, 1);
        assert_eq!(parser.pps.slice_group_map_type, 0); // Interleaved
        assert!(!parser.pps.slice_group_info.is_empty());
    }

    // ============================================
    // Profile-Specific Tests
    // ============================================

    #[test]
    fn test_pps_parse_baseline_profile() {
        let data = create_baseline_pps();
        let bytes_reader = BytesReader::new(data);
        let mut parser = PpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.pps.pic_parameter_set_id, 0);
        assert_eq!(parser.pps.seq_parameter_set_id, 0);
        assert_eq!(parser.pps.entropy_coding_mode_flag, 0); // CAVLC for Baseline
        assert_eq!(parser.pps.num_slice_groups_minus1, 0);
    }

    #[test]
    fn test_pps_parse_high_profile() {
        let data = create_high_pps_cabac();
        let bytes_reader = BytesReader::new(data);
        let mut parser = PpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.pps.pic_parameter_set_id, 1);
        assert_eq!(parser.pps.seq_parameter_set_id, 0);
        assert_eq!(parser.pps.entropy_coding_mode_flag, 1); // CABAC for High profile
    }

    // ============================================
    // Slice Group Info Tests
    // ============================================

    #[test]
    fn test_pps_slice_group_info_interleaved() {
        let data = create_pps_with_slice_groups();
        let bytes_reader = BytesReader::new(data);
        let mut parser = PpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_ok());
        assert_eq!(parser.pps.num_slice_groups_minus1, 1);
        assert_eq!(parser.pps.slice_group_map_type, 0); // Interleaved
        // Should have run_length_minus1 values for each slice group
        assert!(parser.pps.slice_group_info.len() >= 2);
    }

    // ============================================
    // Error Handling Tests
    // ============================================

    #[test]
    fn test_pps_parse_not_enough_data() {
        let mut data = BytesMut::new();
        // Incomplete PPS - only pic_parameter_set_id
        data.extend_from_slice(&[0x80]);
        let bytes_reader = BytesReader::new(data);
        let mut parser = PpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_err());
    }

    #[test]
    fn test_pps_parse_empty_data() {
        let bytes_reader = BytesReader::new(BytesMut::new());
        let mut parser = PpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_err());
    }

    #[test]
    fn test_pps_parse_malformed_uev() {
        let mut data = BytesMut::new();
        // Invalid ue(v) encoding - incomplete Exp-Golomb code
        // This should cause a parsing error
        data.extend_from_slice(&[0x00]); // Leading zeros without following bits
        let bytes_reader = BytesReader::new(data);
        let mut parser = PpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_err());
    }

    #[test]
    fn test_pps_parse_malformed_slice_group_info() {
        let mut data = BytesMut::new();
        // pic_parameter_set_id = 0
        data.extend_from_slice(&[0x80]);
        // seq_parameter_set_id = 0
        data.extend_from_slice(&[0x80]);
        // entropy_coding_mode_flag = 0, bottom_field_pic_order_in_frame_present_flag = 0
        data.extend_from_slice(&[0x00]);
        // num_slice_groups_minus1 = 1 (indicating slice groups present)
        data.extend_from_slice(&[0x40]);
        // slice_group_map_type = 0 (interleaved)
        data.extend_from_slice(&[0x80]);
        // Incomplete slice group info - missing required run_length_minus1 values
        // This should cause an error when trying to read the missing data
        let bytes_reader = BytesReader::new(data);
        let mut parser = PpsParser::new(bytes_reader);
        let result = parser.parse();

        assert!(result.is_err());
    }
}
