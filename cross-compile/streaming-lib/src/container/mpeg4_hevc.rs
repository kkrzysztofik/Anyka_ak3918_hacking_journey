use {
    super::errors::Mpeg4AvcHevcError, crate::bytesio::bytes_reader::BytesReader,
    byteorder::BigEndian,
};
#[allow(dead_code)]
#[derive(Default)]

pub struct Mpeg4Hevc {
    configuration_version: u8, // 1-only
    general_profile_space: u8, // 2bit,[0,3]
    general_tier_flag: u8,     // 1bit,[0,1]
    general_profile_idc: u8,   // 5bit,[0,31]
    general_profile_compatibility_flags: u32,
    general_constraint_indicator_flags: u64,
    general_level_idc: u8,
    min_spatial_segmentation_idc: u16,
    parallelism_type: u8,        // 2bit,[0,3]
    chroma_format: u8,           // 2bit,[0,3]
    bit_depth_luma_minus8: u8,   // 3bit,[0,7]
    bit_depth_chroma_minus8: u8, // 3bit,[0,7]
    avg_frame_rate: u16,
    constant_frame_rate: u8,   // 2bit,[0,3]
    num_temporal_layers: u8,   // 3bit,[0,7]
    temporal_id_nested: u8,    // 1bit,[0,1]
    length_size_minus_one: u8, // 2bit,[0,3]
}

#[derive(Default)]
pub struct Mpeg4HevcProcessor {
    pub mpeg4_hevc: Mpeg4Hevc,
}

impl Mpeg4HevcProcessor {
    pub fn decoder_configuration_record_load(
        &mut self,
        bytes_reader: &mut BytesReader,
    ) -> Result<&mut Self, Mpeg4AvcHevcError> {
        self.mpeg4_hevc.configuration_version = bytes_reader.read_u8()?;
        let byte_1 = bytes_reader.read_u8()?;
        self.mpeg4_hevc.general_profile_space = (byte_1 >> 6) & 0x03;
        self.mpeg4_hevc.general_tier_flag = (byte_1 >> 5) & 0x01;
        self.mpeg4_hevc.general_profile_idc = byte_1 & 0x1F;
        self.mpeg4_hevc.general_profile_compatibility_flags =
            bytes_reader.read_u32::<BigEndian>()?;
        self.mpeg4_hevc.general_constraint_indicator_flags =
            bytes_reader.read_u48::<BigEndian>()?;
        self.mpeg4_hevc.general_level_idc = bytes_reader.read_u8()?;
        self.mpeg4_hevc.min_spatial_segmentation_idc =
            bytes_reader.read_u16::<BigEndian>()? & 0x0FFF;
        self.mpeg4_hevc.parallelism_type = bytes_reader.read_u8()? & 0x03;
        self.mpeg4_hevc.chroma_format = bytes_reader.read_u8()? & 0x03;
        self.mpeg4_hevc.bit_depth_luma_minus8 = bytes_reader.read_u8()? & 0x07;
        self.mpeg4_hevc.bit_depth_chroma_minus8 = bytes_reader.read_u8()? & 0x07;
        self.mpeg4_hevc.avg_frame_rate = bytes_reader.read_u16::<BigEndian>()?;

        let packed = bytes_reader.read_u8()?;
        self.mpeg4_hevc.constant_frame_rate = (packed >> 6) & 0x03;
        self.mpeg4_hevc.num_temporal_layers = (packed >> 3) & 0x07;
        self.mpeg4_hevc.temporal_id_nested = (packed >> 2) & 0x01;
        self.mpeg4_hevc.length_size_minus_one = packed & 0x03;

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytesio::bytes_reader::BytesReader;
    use bytes::BytesMut;

    #[test]
    fn test_decoder_configuration_record_load_success() {
        let data = BytesMut::from(
            &[
                0x01, // configuration_version
                0xBA, // profile_space=2, tier=1, profile_idc=0x1A
                0x11, 0x22, 0x33, 0x44, // compatibility flags
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, // constraint indicator flags (48-bit)
                0x2A, // level_idc
                0xFA, 0xBC, // min_spatial_segmentation_idc (masked to 0x0ABC)
                0xF2, // parallelism_type (0x02)
                0x05, // chroma_format (0x01)
                0xF9, // bit_depth_luma_minus8 (0x01)
                0xFC, // bit_depth_chroma_minus8 (0x04)
                0x12, 0x34, // avg_frame_rate
                0xAF, // packed fields
            ][..],
        );

        let mut reader = BytesReader::new(data);
        let mut processor = Mpeg4HevcProcessor::default();
        processor
            .decoder_configuration_record_load(&mut reader)
            .unwrap();

        let hevc = &processor.mpeg4_hevc;
        assert_eq!(hevc.configuration_version, 1);
        assert_eq!(hevc.general_profile_space, 2);
        assert_eq!(hevc.general_tier_flag, 1);
        assert_eq!(hevc.general_profile_idc, 0x1A);
        assert_eq!(hevc.general_profile_compatibility_flags, 0x11223344);
        assert_eq!(hevc.general_constraint_indicator_flags, 0x010203040506);
        assert_eq!(hevc.general_level_idc, 0x2A);
        assert_eq!(hevc.min_spatial_segmentation_idc, 0x0ABC);
        assert_eq!(hevc.parallelism_type, 0x02);
        assert_eq!(hevc.chroma_format, 0x01);
        assert_eq!(hevc.bit_depth_luma_minus8, 0x01);
        assert_eq!(hevc.bit_depth_chroma_minus8, 0x04);
        assert_eq!(hevc.avg_frame_rate, 0x1234);
        assert_eq!(hevc.constant_frame_rate, 0x02);
        assert_eq!(hevc.num_temporal_layers, 0x05);
        assert_eq!(hevc.temporal_id_nested, 0x01);
        assert_eq!(hevc.length_size_minus_one, 0x03);
    }

    #[test]
    fn test_decoder_configuration_record_load_truncated() {
        let data = BytesMut::from(&[0x01, 0x00][..]);
        let mut reader = BytesReader::new(data);
        let mut processor = Mpeg4HevcProcessor::default();

        let result = processor.decoder_configuration_record_load(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_decoder_configuration_record_load_truncated_after_version() {
        // Only version byte provided, needs at least 2 bytes (version + profile byte)
        let data = BytesMut::from(&[0x01][..]);
        let mut reader = BytesReader::new(data);
        let mut processor = Mpeg4HevcProcessor::default();

        let result = processor.decoder_configuration_record_load(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_decoder_configuration_record_load_truncated_after_profile() {
        // Version + profile byte, but missing compatibility flags (4 bytes)
        let data = BytesMut::from(&[0x01, 0xBA][..]);
        let mut reader = BytesReader::new(data);
        let mut processor = Mpeg4HevcProcessor::default();

        let result = processor.decoder_configuration_record_load(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_decoder_configuration_record_load_truncated_after_constraint() {
        // Version + profile + compatibility flags, but missing constraint flags (6 bytes)
        let data = BytesMut::from(&[0x01, 0xBA, 0x11, 0x22, 0x33, 0x44][..]);
        let mut reader = BytesReader::new(data);
        let mut processor = Mpeg4HevcProcessor::default();

        let result = processor.decoder_configuration_record_load(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_decoder_configuration_record_load_truncated_after_level() {
        // Valid up to level_idc but missing min_spatial_segmentation_idc
        let data = BytesMut::from(
            &[
                0x01, // configuration_version
                0xBA, // profile_space=2, tier=1, profile_idc=0x1A
                0x11, 0x22, 0x33, 0x44, // compatibility flags
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, // constraint indicator flags (48-bit)
                0x2A, // level_idc
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let mut processor = Mpeg4HevcProcessor::default();

        let result = processor.decoder_configuration_record_load(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_decoder_configuration_record_load_truncated_after_spatial_segmentation() {
        // Valid up to min_spatial_segmentation_idc but missing parallelism_type
        let data = BytesMut::from(
            &[
                0x01, // configuration_version
                0xBA, // profile_space=2, tier=1, profile_idc=0x1A
                0x11, 0x22, 0x33, 0x44, // compatibility flags
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, // constraint indicator flags (48-bit)
                0x2A, // level_idc
                0xFA, 0xBC, // min_spatial_segmentation_idc
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let mut processor = Mpeg4HevcProcessor::default();

        let result = processor.decoder_configuration_record_load(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_decoder_configuration_record_load_truncated_before_final_packed() {
        // Valid up to avg_frame_rate but missing final packed byte
        let data = BytesMut::from(
            &[
                0x01, // configuration_version
                0xBA, // profile_space=2, tier=1, profile_idc=0x1A
                0x11, 0x22, 0x33, 0x44, // compatibility flags
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, // constraint indicator flags (48-bit)
                0x2A, // level_idc
                0xFA, 0xBC, // min_spatial_segmentation_idc (masked to 0x0ABC)
                0xF2, // parallelism_type (0x02)
                0x05, // chroma_format (0x01)
                0xF9, // bit_depth_luma_minus8 (0x01)
                0xFC, // bit_depth_chroma_minus8 (0x04)
                0x12, 0x34, // avg_frame_rate
            ][..],
        );
        let mut reader = BytesReader::new(data);
        let mut processor = Mpeg4HevcProcessor::default();

        let result = processor.decoder_configuration_record_load(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_decoder_configuration_record_load_empty_data() {
        let data = BytesMut::new();
        let mut reader = BytesReader::new(data);
        let mut processor = Mpeg4HevcProcessor::default();

        let result = processor.decoder_configuration_record_load(&mut reader);
        assert!(result.is_err());
    }
}
