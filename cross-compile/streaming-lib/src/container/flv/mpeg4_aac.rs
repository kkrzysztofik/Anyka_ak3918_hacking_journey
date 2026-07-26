use {
    crate::container::errors::{MpegAacError, MpegErrorValue},
    crate::io::{
        bits_reader::BitsReader, bits_writer::BitsWriter, bytes_reader::BytesReader,
        bytes_writer::BytesWriter,
    },
    bytes::BytesMut,
};

const AAC_FREQUENCE_SIZE: usize = 13;
const AAC_FREQUENCE: [u32; AAC_FREQUENCE_SIZE] = [
    96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350,
];

const SYNC_EXTENSION_TYPE_SBR: u64 = 0x2B7;
const SYNC_EXTENSION_TYPE_PS: u64 = 0x548;

#[derive(Debug, Clone, Default)]
pub struct Mpeg4Aac {
    pub object_type: u8,
    pub sampling_frequency_index: u8,
    pub channel_configuration: u8,

    pub sampling_frequency: u32,
    pub channels: u8,
    pub sbr: usize,
    pub ps: usize,
    pub pce: BytesMut,
    pub npce: usize,
}

impl Mpeg4Aac {
    pub fn new(
        object_type: u8,
        sampling_frequency: u32,
        channel_configuration: u8,
    ) -> Result<Self, MpegAacError> {
        let sampling_frequency_index = match sampling_frequency {
            96000 => 0,
            88200 => 1,
            64000 => 2,
            48000 => 3,
            44100 => 4,
            32000 => 5,
            24000 => 6,
            22050 => 7,
            16000 => 8,
            12000 => 9,
            11025 => 10,
            8000 => 11,
            7350 => 12,
            _ => {
                return Err(MpegAacError {
                    value: MpegErrorValue::NotSupportedSamplingFrequency,
                });
            }
        };

        Ok(Self {
            object_type,
            sampling_frequency_index,
            channel_configuration,
            sampling_frequency,
            ..Default::default()
        })
    }
    // 11 90
    // 00010 0011 0010 000
    // 2   3  2
    //https://wiki.multimedia.cx/index.php?title=MPEG-4_Audio#Audio_Specific_Config
    pub fn gen_audio_specific_config(&self) -> Result<BytesMut, MpegAacError> {
        let mut writer = BytesWriter::default();
        writer.write_u8(self.object_type << 3 | (self.sampling_frequency_index >> 1))?;
        writer.write_u8(
            (self.sampling_frequency_index & 0x01) << 7 | (self.channel_configuration << 3),
        )?;
        Ok(writer.extract_current_bytes())
    }
}

pub struct Mpeg4AacProcessor {
    pub bytes_reader: BytesReader,
    pub bytes_writer: BytesWriter,
    pub bits_reader: BitsReader,
    pub mpeg4_aac: Mpeg4Aac,
}

impl Default for Mpeg4AacProcessor {
    fn default() -> Self {
        Self::new()
    }
}
//https://blog.csdn.net/coloriy/article/details/90511746
impl Mpeg4AacProcessor {
    pub fn new() -> Self {
        Self {
            bytes_reader: BytesReader::new(BytesMut::new()),
            bytes_writer: BytesWriter::new(),
            bits_reader: BitsReader::new(BytesReader::new(BytesMut::new())),
            mpeg4_aac: Mpeg4Aac::default(),
        }
    }

    pub fn extend_data(&mut self, data: BytesMut) -> &mut Self {
        self.bytes_reader.extend_from_slice(&data[..]);
        self
    }

    pub fn audio_specific_config_load(&mut self) -> Result<&mut Self, MpegAacError> {
        let byte_0 = self.bytes_reader.read_u8()?;
        self.mpeg4_aac.object_type = (byte_0 >> 3) & 0x1F;

        let byte_1 = self.bytes_reader.read_u8()?;
        self.mpeg4_aac.sampling_frequency_index = ((byte_0 & 0x07) << 1) | ((byte_1 >> 7) & 0x01);
        self.mpeg4_aac.channel_configuration = (byte_1 >> 3) & 0x0F;
        self.mpeg4_aac.channels = self.mpeg4_aac.channel_configuration;
        self.mpeg4_aac.sampling_frequency =
            Self::get_freq_by_index(self.mpeg4_aac.sampling_frequency_index)?;

        self.bytes_reader.extract_remaining_bytes();

        Ok(self)
    }

    fn get_freq_by_index(index: u8) -> Result<u32, MpegAacError> {
        AAC_FREQUENCE
            .get(index as usize)
            .copied()
            .ok_or(MpegAacError {
                value: MpegErrorValue::NotSupportedSamplingFrequency,
            })
    }

    fn handle_sbr_ps_extension(
        &mut self,
        extension_audio_object_type: &mut u8,
        extension_sampling_frequency_index: &mut u32,
    ) -> Result<(), MpegAacError> {
        if *extension_audio_object_type != 5 || self.bits_reader.len() < 16 {
            return Ok(());
        }

        let sync_extension_type = self.bits_reader.read_n_bits(11)?;
        if sync_extension_type != SYNC_EXTENSION_TYPE_SBR {
            return Ok(());
        }

        *extension_audio_object_type = self.get_audio_object_type()?;

        match *extension_audio_object_type {
            5 => self.handle_sbr_extension(extension_sampling_frequency_index)?,
            22 => self.handle_sbr_extension_type22(extension_sampling_frequency_index)?,
            _ => {}
        }

        Ok(())
    }

    fn handle_sbr_extension(
        &mut self,
        extension_sampling_frequency_index: &mut u32,
    ) -> Result<(), MpegAacError> {
        self.mpeg4_aac.sbr = self.bits_reader.read_n_bits(1)? as usize;
        if self.mpeg4_aac.sbr > 0 {
            *extension_sampling_frequency_index = self.get_sampling_frequency()?;
            if self.bits_reader.len() >= 12 {
                let sync_extension_type = self.bits_reader.read_n_bits(11)?;
                if sync_extension_type == SYNC_EXTENSION_TYPE_PS {
                    self.mpeg4_aac.ps = self.bits_reader.read_n_bits(1)? as usize;
                }
            }
        }
        Ok(())
    }

    fn handle_sbr_extension_type22(
        &mut self,
        extension_sampling_frequency_index: &mut u32,
    ) -> Result<(), MpegAacError> {
        self.mpeg4_aac.sbr = self.bits_reader.read_n_bits(1)? as usize;
        if self.mpeg4_aac.sbr > 0 {
            *extension_sampling_frequency_index = self.get_sampling_frequency()?;
        }
        self.bits_reader.read_n_bits(4)?;
        Ok(())
    }

    fn process_extension_config(&mut self) -> Result<(), MpegAacError> {
        let ep_config = self.bits_reader.read_n_bits(2)?;
        if matches!(ep_config, 2 | 3) {
            return Err(MpegAacError {
                value: MpegErrorValue::ShouldNotComeHere,
            });
        }
        Ok(())
    }

    fn load_specific_config_by_type(&mut self) -> Result<(), MpegAacError> {
        match self.mpeg4_aac.object_type {
            1..=7 | 17 | 19..=23 => self.ga_specific_config_load(),
            8 => self.celp_specific_config_load(),
            _ => Ok(()),
        }
    }

    pub fn audio_specific_config_load2(&mut self) -> Result<(), MpegAacError> {
        let remain_bytes = self.bytes_reader.extract_remaining_bytes();
        self.bits_reader.extend_data(remain_bytes);

        self.mpeg4_aac.object_type = self.get_audio_object_type()?;
        let sampling_frequency = self.get_sampling_frequency()?;

        if sampling_frequency <= 0x0F {
            self.mpeg4_aac.sampling_frequency_index = sampling_frequency as u8;
            self.mpeg4_aac.sampling_frequency = Self::get_freq_by_index(sampling_frequency as u8)?;
        } else {
            self.mpeg4_aac.sampling_frequency_index = 0x0F;
            self.mpeg4_aac.sampling_frequency = sampling_frequency;
        }
        self.mpeg4_aac.channel_configuration = self.bits_reader.read_n_bits(4)? as u8;

        let mut extension_audio_object_type: u8;
        let mut extension_sampling_frequency_index: u32 = 0;

        if self.mpeg4_aac.object_type == 5 || self.mpeg4_aac.object_type == 29 {
            extension_audio_object_type = 5;
            self.mpeg4_aac.sbr = 1;
            if self.mpeg4_aac.object_type == 29 {
                self.mpeg4_aac.ps = 1;
            }
            extension_sampling_frequency_index = self.get_sampling_frequency()?;
            self.mpeg4_aac.object_type = self.get_audio_object_type()?;
        } else {
            extension_audio_object_type = 0;
        }

        self.load_specific_config_by_type()?;

        if matches!(self.mpeg4_aac.object_type, 17 | 19..=27 | 39) {
            self.process_extension_config()?;
        }

        self.handle_sbr_ps_extension(
            &mut extension_audio_object_type,
            &mut extension_sampling_frequency_index,
        )?;

        self.bits_reader.bits_alignment_8();

        let _ = extension_audio_object_type;
        let _ = extension_sampling_frequency_index;

        Ok(())
    }

    pub fn celp_specific_config_load(&mut self) -> Result<(), MpegAacError> {
        if self.bits_reader.read_n_bits(1)? > 0 {
            let excitation_mode = self.bits_reader.read_n_bits(1)?;
            self.bits_reader.read_n_bits(1)?;
            self.bits_reader.read_n_bits(1)?;

            match excitation_mode {
                1 => {
                    self.bits_reader.read_n_bits(3)?;
                }
                0 => {
                    self.bits_reader.read_n_bits(5)?;
                    self.bits_reader.read_n_bits(2)?;
                    self.bits_reader.read_n_bits(1)?;
                }
                _ => {}
            }
        } else {
            self.bits_reader.read_n_bits(1)?;
            self.bits_reader.read_n_bits(2)?;
        }

        Ok(())
    }

    fn handle_extension_flag_type22(&mut self) -> Result<(), MpegAacError> {
        self.bits_reader.read_n_bits(5)?;
        self.bits_reader.read_n_bits(11)?;
        Ok(())
    }

    fn handle_extension_flag_type_17_19_20_23(&mut self) -> Result<(), MpegAacError> {
        self.bits_reader.read_n_bits(1)?;
        self.bits_reader.read_n_bits(1)?;
        self.bits_reader.read_n_bits(1)?;
        Ok(())
    }

    pub fn ga_specific_config_load(&mut self) -> Result<(), MpegAacError> {
        self.bits_reader.read_n_bits(1)?;

        if self.bits_reader.read_n_bits(1)? > 0 {
            self.bits_reader.read_n_bits(14)?;
        }
        let extension_flag: u64 = self.bits_reader.read_n_bits(1)?;

        if self.mpeg4_aac.channel_configuration == 0 {
            self.pce_load()?;
        }

        if self.mpeg4_aac.object_type == 6 || self.mpeg4_aac.object_type == 20 {
            self.bits_reader.read_n_bits(3)?;
        }

        if extension_flag > 0 {
            match self.mpeg4_aac.object_type {
                22 => self.handle_extension_flag_type22()?,
                17 | 19 | 20 | 23 => self.handle_extension_flag_type_17_19_20_23()?,
                _ => {}
            }
            self.bits_reader.read_n_bits(1)?;
        }

        Ok(())
    }

    fn mpeg4_bits_copy(
        &mut self,
        writer: &mut BitsWriter,
        read_len: usize,
    ) -> Result<u64, MpegAacError> {
        let data = self.bits_reader.read_n_bits(read_len)?;
        writer.write_n_bits(data, read_len)?;
        Ok(data)
    }

    fn read_channel_element(&mut self, pce_bits_vec: &mut BitsWriter) -> Result<u64, MpegAacError> {
        let cpe = self.mpeg4_bits_copy(pce_bits_vec, 1)?;
        self.mpeg4_bits_copy(pce_bits_vec, 4)?;
        Ok(cpe)
    }

    fn count_channels_for_element(&self, cpe: u64) -> u8 {
        if cpe > 0 || self.mpeg4_aac.ps > 0 {
            2
        } else {
            1
        }
    }

    pub fn pce_load(&mut self) -> Result<u8, MpegAacError> {
        let mut pce_bits_vec = BitsWriter::new(BytesWriter::new());
        pce_bits_vec.write_bytes(self.mpeg4_aac.pce.clone())?;

        self.mpeg4_aac.channels = 0;

        let element_instance_tag: u64 = self.mpeg4_bits_copy(&mut pce_bits_vec, 4)?;
        let object_type: u64 = self.mpeg4_bits_copy(&mut pce_bits_vec, 2)?;
        let sampling_frequency_index: u64 = self.mpeg4_bits_copy(&mut pce_bits_vec, 4)?;
        let num_front_channel_elements: u64 = self.mpeg4_bits_copy(&mut pce_bits_vec, 4)?;
        let num_side_channel_elements: u64 = self.mpeg4_bits_copy(&mut pce_bits_vec, 4)?;
        let num_back_channel_elements: u64 = self.mpeg4_bits_copy(&mut pce_bits_vec, 4)?;
        let num_lfe_channel_elements: u64 = self.mpeg4_bits_copy(&mut pce_bits_vec, 2)?;
        let num_assoc_data_elements: u64 = self.mpeg4_bits_copy(&mut pce_bits_vec, 3)?;
        let num_valid_cc_elements: u64 = self.mpeg4_bits_copy(&mut pce_bits_vec, 4)?;

        for _ in 0..3 {
            if self.mpeg4_bits_copy(&mut pce_bits_vec, 1)? > 0 {
                self.mpeg4_bits_copy(&mut pce_bits_vec, 4)?;
            }
        }

        for _ in 0..num_front_channel_elements {
            let cpe = self.read_channel_element(&mut pce_bits_vec)?;
            self.mpeg4_aac.channels += self.count_channels_for_element(cpe);
        }

        for _ in 0..num_side_channel_elements {
            let cpe = self.read_channel_element(&mut pce_bits_vec)?;
            self.mpeg4_aac.channels += self.count_channels_for_element(cpe);
        }

        for _ in 0..num_back_channel_elements {
            let cpe = self.read_channel_element(&mut pce_bits_vec)?;
            self.mpeg4_aac.channels += self.count_channels_for_element(cpe);
        }

        for _ in 0..num_lfe_channel_elements {
            self.mpeg4_bits_copy(&mut pce_bits_vec, 4)?;
            self.mpeg4_aac.channels += 1;
        }

        for _ in 0..num_assoc_data_elements {
            self.mpeg4_bits_copy(&mut pce_bits_vec, 4)?;
        }

        for _ in 0..num_valid_cc_elements {
            self.read_channel_element(&mut pce_bits_vec)?;
        }

        pce_bits_vec.bits_alignment_8()?;
        self.bits_reader.bits_alignment_8();

        let comment_field_bytes: u64 = self.mpeg4_bits_copy(&mut pce_bits_vec, 8)?;

        for _ in 0..comment_field_bytes {
            self.mpeg4_bits_copy(&mut pce_bits_vec, 8)?;
        }

        let rv = pce_bits_vec.len().div_ceil(8);

        let _ = element_instance_tag;
        let _ = object_type;
        let _ = sampling_frequency_index;

        Ok(rv as u8)
    }

    pub fn get_audio_object_type(&mut self) -> Result<u8, MpegAacError> {
        let mut audio_object_type: u64;

        audio_object_type = self.bits_reader.read_n_bits(5)?;
        if 31 == audio_object_type {
            audio_object_type = 32 + self.bits_reader.read_n_bits(6)?;
        }

        Ok(audio_object_type as u8)
    }

    pub fn get_sampling_frequency(&mut self) -> Result<u32, MpegAacError> {
        let mut sampling_frequency_index: u64;

        sampling_frequency_index = self.bits_reader.read_n_bits(4)?;
        if sampling_frequency_index == 0x0F {
            sampling_frequency_index = self.bits_reader.read_n_bits(24)?;
        }

        Ok(sampling_frequency_index as u32)
    }

    pub fn adts_save(&mut self) -> Result<(), MpegAacError> {
        let id = 0; // 0-MPEG4/1-MPEG2
        let len = (self.bytes_reader.len() + 7) as u32;
        self.bytes_writer.write_u8(0xFF)?; //0
        self.bytes_writer.write_u8(
            0xF0 /* 12-syncword */ | (id << 3)/*1-ID*/| 0x01, /*1-protection_absent*/
        )?; //1

        let profile = self.mpeg4_aac.object_type;
        let sampling_frequency_index = self.mpeg4_aac.sampling_frequency_index;
        let channel_configuration = self.mpeg4_aac.channel_configuration;
        self.bytes_writer.write_u8(
            ((profile - 1) << 6)
                | ((sampling_frequency_index & 0x0F) << 2)
                | ((channel_configuration >> 2) & 0x01),
        )?; //2

        self.bytes_writer
            .write_u8(((channel_configuration & 0x03) << 6) | ((len >> 11) as u8 & 0x03))?; //3
        self.bytes_writer.write_u8((len >> 3) as u8)?; //4
        self.bytes_writer
            .write_u8((((len & 0x07) as u8) << 5) | 0x1F)?; //5
        self.bytes_writer.write_u8(0xFC)?; //6

        self.bytes_writer
            .write(&self.bytes_reader.extract_remaining_bytes()[..])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== AAC_FREQUENCE Constant Tests ==========

    #[test]
    fn test_aac_frequence_constant() {
        assert_eq!(AAC_FREQUENCE_SIZE, 13);
        assert_eq!(AAC_FREQUENCE[0], 96000);
        assert_eq!(AAC_FREQUENCE[4], 44100);
        assert_eq!(AAC_FREQUENCE[11], 8000);
        assert_eq!(AAC_FREQUENCE[12], 7350);
    }

    // ========== Mpeg4Aac Construction Tests ==========

    #[test]
    fn test_mpeg4_aac_default() {
        let aac = Mpeg4Aac::default();
        assert_eq!(aac.object_type, 0);
        assert_eq!(aac.sampling_frequency_index, 0);
        assert_eq!(aac.channel_configuration, 0);
        assert_eq!(aac.sampling_frequency, 0);
        assert_eq!(aac.channels, 0);
    }

    #[test]
    fn test_mpeg4_aac_new_44100hz_stereo() {
        let aac = Mpeg4Aac::new(2, 44100, 2).unwrap();
        assert_eq!(aac.object_type, 2);
        assert_eq!(aac.sampling_frequency_index, 4);
        assert_eq!(aac.channel_configuration, 2);
        assert_eq!(aac.sampling_frequency, 44100);
    }

    #[test]
    fn test_mpeg4_aac_new_48000hz_stereo() {
        let aac = Mpeg4Aac::new(2, 48000, 2).unwrap();
        assert_eq!(aac.object_type, 2);
        assert_eq!(aac.sampling_frequency_index, 3);
        assert_eq!(aac.sampling_frequency, 48000);
    }

    #[test]
    fn test_mpeg4_aac_new_all_supported_frequencies() {
        let frequencies = [
            96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350,
        ];
        for (index, freq) in frequencies.iter().enumerate() {
            let aac = Mpeg4Aac::new(2, *freq, 2).unwrap();
            assert_eq!(aac.sampling_frequency_index, index as u8);
            assert_eq!(aac.sampling_frequency, *freq);
        }
    }

    #[test]
    fn test_mpeg4_aac_new_unsupported_frequency() {
        let result = Mpeg4Aac::new(2, 12345, 2);
        assert!(result.is_err());
    }

    // ========== Audio Specific Config Generation Tests ==========

    #[test]
    fn test_gen_audio_specific_config_aac_lc_44100_stereo() {
        // AAC-LC (object_type=2), 44100Hz (index=4), stereo (channels=2)
        let aac = Mpeg4Aac::new(2, 44100, 2).unwrap();
        let config = aac.gen_audio_specific_config().unwrap();
        assert_eq!(config.len(), 2);
        // Verify bit encoding:
        // object_type(2) << 3 | sampling_frequency_index(4) >> 1 = 16 | 2 = 18 = 0x12
        // (sampling_frequency_index(4) & 0x01) << 7 | channel_configuration(2) << 3 = 0 | 16 = 0x10
        assert_eq!(config[0], 0x12);
        assert_eq!(config[1], 0x10);
    }

    #[test]
    fn test_gen_audio_specific_config_aac_lc_48000_stereo() {
        // AAC-LC (object_type=2), 48000Hz (index=3), stereo (channels=2)
        let aac = Mpeg4Aac::new(2, 48000, 2).unwrap();
        let config = aac.gen_audio_specific_config().unwrap();
        assert_eq!(config.len(), 2);
        // object_type(2) << 3 | sampling_frequency_index(3) >> 1 = 16 | 1 = 17 = 0x11
        // (sampling_frequency_index(3) & 0x01) << 7 | channel_configuration(2) << 3 = 128 | 16 = 144 = 0x90
        assert_eq!(config[0], 0x11);
        assert_eq!(config[1], 0x90);
    }

    // ========== Mpeg4AacProcessor Tests ==========

    #[test]
    fn test_mpeg4_aac_processor_new() {
        let processor = Mpeg4AacProcessor::new();
        assert_eq!(processor.mpeg4_aac.object_type, 0);
    }

    #[test]
    fn test_mpeg4_aac_processor_default() {
        let processor = Mpeg4AacProcessor::default();
        assert_eq!(processor.mpeg4_aac.object_type, 0);
    }

    #[test]
    fn test_mpeg4_aac_processor_extend_data() {
        let mut processor = Mpeg4AacProcessor::new();
        let data = BytesMut::from(&[0x11, 0x90][..]);
        processor.extend_data(data);
        // Data should be extended
        assert!(!processor.bytes_reader.is_empty());
    }

    #[test]
    fn test_audio_specific_config_load() {
        let mut processor = Mpeg4AacProcessor::new();
        // AAC-LC, 48000Hz, stereo: 0x11 0x90
        let data = BytesMut::from(&[0x11, 0x90][..]);
        processor.extend_data(data);

        let result = processor.audio_specific_config_load();
        assert!(result.is_ok());

        // Verify parsed values
        // 0x11 = 0001 0001, object_type = 00010 = 2
        // sampling_frequency_index = 011 | 1 = 0x3 (48000Hz)
        // channel_configuration = 0010 = 2 (stereo)
        assert_eq!(processor.mpeg4_aac.object_type, 2);
        assert_eq!(processor.mpeg4_aac.sampling_frequency_index, 3);
        assert_eq!(processor.mpeg4_aac.channel_configuration, 2);
        assert_eq!(processor.mpeg4_aac.sampling_frequency, 48000);
    }

    #[test]
    fn test_audio_specific_config_load_44100_stereo() {
        let mut processor = Mpeg4AacProcessor::new();
        // AAC-LC, 44100Hz, stereo: 0x12 0x10
        let data = BytesMut::from(&[0x12, 0x10][..]);
        processor.extend_data(data);

        let result = processor.audio_specific_config_load();
        assert!(result.is_ok());

        assert_eq!(processor.mpeg4_aac.object_type, 2);
        assert_eq!(processor.mpeg4_aac.sampling_frequency_index, 4);
        assert_eq!(processor.mpeg4_aac.channel_configuration, 2);
        assert_eq!(processor.mpeg4_aac.sampling_frequency, 44100);
    }

    // ========== Clone and Debug Tests ==========

    #[test]
    fn test_mpeg4_aac_clone() {
        let aac = Mpeg4Aac::new(2, 44100, 2).unwrap();
        let cloned = aac.clone();
        assert_eq!(cloned.object_type, aac.object_type);
        assert_eq!(cloned.sampling_frequency, aac.sampling_frequency);
    }

    #[test]
    fn test_mpeg4_aac_debug() {
        let aac = Mpeg4Aac::default();
        let debug_str = format!("{:?}", aac);
        assert!(debug_str.contains("Mpeg4Aac"));
        assert!(debug_str.contains("object_type"));
    }

    // ========== Round-Trip Tests ==========

    #[test]
    fn test_audio_specific_config_roundtrip() {
        // Create AAC and generate config
        let original = Mpeg4Aac::new(2, 44100, 2).unwrap();
        let config = original.gen_audio_specific_config().unwrap();

        // Parse the config back
        let mut processor = Mpeg4AacProcessor::new();
        processor.extend_data(config);
        processor.audio_specific_config_load().unwrap();

        // Verify round-trip
        assert_eq!(processor.mpeg4_aac.object_type, original.object_type);
        assert_eq!(
            processor.mpeg4_aac.sampling_frequency_index,
            original.sampling_frequency_index
        );
        assert_eq!(
            processor.mpeg4_aac.channel_configuration,
            original.channel_configuration
        );
    }

    #[test]
    fn test_audio_specific_config_roundtrip_various_frequencies() {
        for freq in [48000u32, 44100, 32000, 22050, 16000, 8000] {
            let original = Mpeg4Aac::new(2, freq, 2).unwrap();
            let config = original.gen_audio_specific_config().unwrap();

            let mut processor = Mpeg4AacProcessor::new();
            processor.extend_data(config);
            processor.audio_specific_config_load().unwrap();

            assert_eq!(processor.mpeg4_aac.sampling_frequency, freq);
        }
    }

    // ========== ADTS Save Tests ==========

    #[test]
    fn test_adts_save_basic() {
        let mut processor = Mpeg4AacProcessor::new();
        // Set up AAC parameters
        processor.mpeg4_aac.object_type = 2; // AAC-LC
        processor.mpeg4_aac.sampling_frequency_index = 4; // 44100Hz
        processor.mpeg4_aac.channel_configuration = 2; // Stereo

        // Add some raw audio data
        let raw_data = BytesMut::from(&[0x21, 0x00, 0x49, 0x90][..]);
        processor.bytes_reader = crate::io::bytes_reader::BytesReader::new(raw_data);

        let result = processor.adts_save();
        assert!(result.is_ok());

        // Verify ADTS header was written (7 bytes + data)
        let output = processor.bytes_writer.extract_current_bytes();
        assert!(output.len() >= 7);
        // Check sync word
        assert_eq!(output[0], 0xFF);
        assert_eq!(output[1] & 0xF0, 0xF0);
    }

    #[test]
    fn test_adts_save_48khz_stereo() {
        let mut processor = Mpeg4AacProcessor::new();
        processor.mpeg4_aac.object_type = 2;
        processor.mpeg4_aac.sampling_frequency_index = 3; // 48000Hz
        processor.mpeg4_aac.channel_configuration = 2;

        let raw_data = BytesMut::from(&[0x00, 0x01, 0x02, 0x03][..]);
        processor.bytes_reader = crate::io::bytes_reader::BytesReader::new(raw_data);

        assert!(processor.adts_save().is_ok());
        let output = processor.bytes_writer.extract_current_bytes();
        assert_eq!(output.len(), 11); // 7 header + 4 data
    }

    #[test]
    fn test_adts_save_mono() {
        let mut processor = Mpeg4AacProcessor::new();
        processor.mpeg4_aac.object_type = 2;
        processor.mpeg4_aac.sampling_frequency_index = 4;
        processor.mpeg4_aac.channel_configuration = 1; // Mono

        let raw_data = BytesMut::from(&[0xAB, 0xCD][..]);
        processor.bytes_reader = crate::io::bytes_reader::BytesReader::new(raw_data);

        assert!(processor.adts_save().is_ok());
    }

    // ========== Get Audio Object Type Tests ==========

    #[test]
    fn test_get_audio_object_type_aac_lc() {
        let mut processor = Mpeg4AacProcessor::new();
        // AAC-LC is type 2: binary 00010
        // 0x11 = 0001 0001 -> object type = 00010 = 2
        let data = BytesMut::from(&[0x11, 0x90][..]);
        // Initialize bits_reader directly for this test as get_audio_object_type uses it
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        let object_type = processor.get_audio_object_type().unwrap();
        assert_eq!(object_type, 2);
    }

    #[test]
    fn test_get_audio_object_type_he_aac() {
        let mut processor = Mpeg4AacProcessor::new();
        // HE-AAC is type 5: binary 00101
        // 0x28 = 0010 1000 -> object type = 00101 = 5
        let data = BytesMut::from(&[0x28, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        let object_type = processor.get_audio_object_type().unwrap();
        assert_eq!(object_type, 5);
    }

    #[test]
    fn test_get_audio_object_type_aac_main() {
        let mut processor = Mpeg4AacProcessor::new();
        // AAC Main is type 1: binary 00001
        // 0x08 = 0000 1000 -> object type = 00001 = 1
        let data = BytesMut::from(&[0x08, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        let object_type = processor.get_audio_object_type().unwrap();
        assert_eq!(object_type, 1);
    }

    // ========== Get Sampling Frequency Tests ==========

    #[test]
    fn test_get_sampling_frequency_48khz() {
        let mut processor = Mpeg4AacProcessor::new();
        // After 5-bit object type, next 4 bits are sampling_frequency_index
        // For index 3 (48000Hz): we need to position bits correctly
        // 0x11 0x90 -> after reading 5 bits (00010), we have 0 0110 -> 0011 = 3
        let data = BytesMut::from(&[0x11, 0x90][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        // Skip object type first
        let _ = processor.get_audio_object_type().unwrap();
        let freq_index = processor.get_sampling_frequency().unwrap();
        assert_eq!(freq_index, 3); // 48000Hz index
    }

    #[test]
    fn test_get_sampling_frequency_44100hz() {
        let mut processor = Mpeg4AacProcessor::new();
        // 0x12 0x10 -> object_type = 2, then sampling_frequency_index = 4 (44100Hz)
        let data = BytesMut::from(&[0x12, 0x10][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        let _ = processor.get_audio_object_type().unwrap();
        let freq_index = processor.get_sampling_frequency().unwrap();
        assert_eq!(freq_index, 4);
    }

    // ========== Various Channel Configuration Tests ==========

    #[test]
    fn test_audio_config_various_channels() {
        for channels in 1..=6u8 {
            let aac = Mpeg4Aac::new(2, 44100, channels).unwrap();
            assert_eq!(aac.channel_configuration, channels);

            let config = aac.gen_audio_specific_config().unwrap();

            let mut processor = Mpeg4AacProcessor::new();
            processor.bits_reader = crate::io::bits_reader::BitsReader::new(
                crate::io::bytes_reader::BytesReader::new(config.clone()),
            );
            processor.extend_data(config);
            processor.audio_specific_config_load().unwrap();

            assert_eq!(processor.mpeg4_aac.channel_configuration, channels);
        }
    }

    // ========== Edge Case Tests ==========

    #[test]
    fn test_mpeg4_aac_all_object_types() {
        // Test valid object types 1-4 (common AAC profiles)
        for obj_type in 1..=4u8 {
            let aac = Mpeg4Aac::new(obj_type, 44100, 2).unwrap();
            assert_eq!(aac.object_type, obj_type);
        }
    }

    #[test]
    fn test_mpeg4_aac_7350hz() {
        // Lowest supported frequency
        let aac = Mpeg4Aac::new(2, 7350, 2).unwrap();
        assert_eq!(aac.sampling_frequency, 7350);
        assert_eq!(aac.sampling_frequency_index, 12);
    }

    #[test]
    fn test_mpeg4_aac_96000hz() {
        // Highest supported frequency
        let aac = Mpeg4Aac::new(2, 96000, 2).unwrap();
        assert_eq!(aac.sampling_frequency, 96000);
        assert_eq!(aac.sampling_frequency_index, 0);
    }

    // ========== Extended audio_specific_config_load2 Tests ==========

    #[test]
    fn test_audio_specific_config_load2_basic_aac_lc() {
        let mut processor = Mpeg4AacProcessor::new();
        // AAC-LC (obj_type=2), 48000Hz (idx=3), stereo (ch=2): 0x11 0x90
        let data = BytesMut::from(&[0x11, 0x90][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        let result = processor.audio_specific_config_load2();
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_aac.object_type, 2);
        assert_eq!(processor.mpeg4_aac.sampling_frequency, 48000);
        assert_eq!(processor.mpeg4_aac.channel_configuration, 2);
    }

    // ========== get_freq_by_index Tests ==========

    #[test]
    fn test_get_freq_by_index_all_valid() {
        for (index, expected_freq) in AAC_FREQUENCE.iter().enumerate() {
            let freq = Mpeg4AacProcessor::get_freq_by_index(index as u8).unwrap();
            assert_eq!(freq, *expected_freq);
        }
    }

    #[test]
    fn test_get_freq_by_index_invalid() {
        let result = Mpeg4AacProcessor::get_freq_by_index(15);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().value,
            MpegErrorValue::NotSupportedSamplingFrequency
        ));
    }

    // ========== Constructor & Configuration Tests ==========

    #[test]
    fn test_mpeg4_aac_new_mono() {
        let aac = Mpeg4Aac::new(2, 44100, 1).unwrap();
        assert_eq!(aac.channel_configuration, 1);
        assert_eq!(aac.sampling_frequency, 44100);
    }

    #[test]
    fn test_mpeg4_aac_new_5_1_channels() {
        let aac = Mpeg4Aac::new(2, 48000, 6).unwrap();
        assert_eq!(aac.channel_configuration, 6);
    }

    #[test]
    fn test_mpeg4_aac_default_values() {
        let aac = Mpeg4Aac::default();
        assert_eq!(aac.sbr, 0);
        assert_eq!(aac.ps, 0);
        assert_eq!(aac.npce, 0);
        assert!(aac.pce.is_empty());
    }

    // ========== gen_audio_specific_config Tests ==========

    #[test]
    fn test_gen_audio_specific_config_all_object_types() {
        for obj_type in 1..=4u8 {
            let aac = Mpeg4Aac::new(obj_type, 44100, 2).unwrap();
            let config = aac.gen_audio_specific_config().unwrap();
            assert_eq!(config.len(), 2);
            // Verify object type is encoded in top 5 bits
            assert_eq!(config[0] >> 3, obj_type);
        }
    }

    #[test]
    fn test_gen_audio_specific_config_various_sample_rates() {
        let sample_rates = [96000, 48000, 44100, 32000, 22050, 16000, 8000, 7350];
        for sample_rate in sample_rates {
            let aac = Mpeg4Aac::new(2, sample_rate, 2).unwrap();
            let config = aac.gen_audio_specific_config().unwrap();
            assert_eq!(config.len(), 2);
        }
    }

    // ========== ADTS Save Tests ==========

    #[test]
    fn test_adts_save_various_configurations() {
        let configs = [
            (2u8, 3u8, 1u8), // AAC-LC, 48kHz, mono
            (2, 4, 2),       // AAC-LC, 44.1kHz, stereo
            (2, 8, 1),       // AAC-LC, 16kHz, mono
        ];

        for (obj_type, freq_idx, channels) in configs {
            let mut processor = Mpeg4AacProcessor::new();
            processor.mpeg4_aac.object_type = obj_type;
            processor.mpeg4_aac.sampling_frequency_index = freq_idx;
            processor.mpeg4_aac.channel_configuration = channels;

            let raw_data = BytesMut::from(&[0x21, 0x10][..]);
            processor.bytes_reader = crate::io::bytes_reader::BytesReader::new(raw_data);

            assert!(processor.adts_save().is_ok());
            let output = processor.bytes_writer.extract_current_bytes();
            // Verify ADTS sync word (0xFFF)
            assert_eq!(output[0], 0xFF);
            assert_eq!(output[1] & 0xF0, 0xF0);
        }
    }

    #[test]
    fn test_adts_save_large_payload() {
        let mut processor = Mpeg4AacProcessor::new();
        processor.mpeg4_aac.object_type = 2;
        processor.mpeg4_aac.sampling_frequency_index = 4;
        processor.mpeg4_aac.channel_configuration = 2;

        let large_data = vec![0xAB; 2048];
        processor.bytes_reader =
            crate::io::bytes_reader::BytesReader::new(BytesMut::from(&large_data[..]));

        assert!(processor.adts_save().is_ok());
        let output = processor.bytes_writer.extract_current_bytes();
        assert_eq!(output.len(), 7 + 2048); // 7-byte ADTS header + payload
    }

    #[test]
    fn test_adts_save_verifies_frame_length_encoding() {
        let mut processor = Mpeg4AacProcessor::new();
        processor.mpeg4_aac.object_type = 2;
        processor.mpeg4_aac.sampling_frequency_index = 4;
        processor.mpeg4_aac.channel_configuration = 2;

        let payload = [0x00; 100];
        processor.bytes_reader =
            crate::io::bytes_reader::BytesReader::new(BytesMut::from(&payload[..]));

        processor.adts_save().unwrap();
        let output = processor.bytes_writer.extract_current_bytes();

        // Frame length = 7 (header) + 100 (payload) = 107
        let frame_len = ((output[3] as u32 & 0x03) << 11)
            | ((output[4] as u32) << 3)
            | ((output[5] as u32) >> 5);
        assert_eq!(frame_len, 107);
    }

    // ========== Error Path Tests ==========

    #[test]
    fn test_audio_specific_config_load_insufficient_data() {
        let mut processor = Mpeg4AacProcessor::new();
        let data = BytesMut::from(&[0x11][..]); // Only 1 byte - not enough
        processor.extend_data(data);

        let result = processor.audio_specific_config_load();
        assert!(result.is_err());
    }

    #[test]
    fn test_mpeg4_aac_new_invalid_frequency_zero() {
        let result = Mpeg4Aac::new(2, 0, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_mpeg4_aac_new_invalid_frequency_between_values() {
        let result = Mpeg4Aac::new(2, 20000, 2); // Between 16000 and 22050
        assert!(result.is_err());
    }

    // ========== Processor Utility Tests ==========

    #[test]
    fn test_extend_data_multiple_calls() {
        let mut processor = Mpeg4AacProcessor::new();
        processor.extend_data(BytesMut::from(&[0x11][..]));
        processor.extend_data(BytesMut::from(&[0x90][..]));
        assert_eq!(processor.bytes_reader.len(), 2);
    }

    // ========== Round-trip Tests ==========

    #[test]
    fn test_audio_specific_config_roundtrip_mono() {
        let original = Mpeg4Aac::new(2, 48000, 1).unwrap();
        let config = original.gen_audio_specific_config().unwrap();

        let mut processor = Mpeg4AacProcessor::new();
        processor.extend_data(config);
        processor.audio_specific_config_load().unwrap();

        assert_eq!(processor.mpeg4_aac.channel_configuration, 1);
    }

    // ========== count_channels_for_element Tests ==========

    #[test]
    fn test_count_channels_for_element_cpe_set() {
        let processor = Mpeg4AacProcessor::new();
        // cpe > 0 should return 2 channels
        assert_eq!(processor.count_channels_for_element(1), 2);
    }

    #[test]
    fn test_count_channels_for_element_cpe_zero_no_ps() {
        let processor = Mpeg4AacProcessor::new();
        // cpe == 0 and ps == 0 should return 1 channel
        assert_eq!(processor.count_channels_for_element(0), 1);
    }

    #[test]
    fn test_count_channels_for_element_cpe_zero_with_ps() {
        let mut processor = Mpeg4AacProcessor::new();
        processor.mpeg4_aac.ps = 1;
        // cpe == 0 but ps > 0 should return 2 channels
        assert_eq!(processor.count_channels_for_element(0), 2);
    }

    // ========== process_extension_config Tests ==========

    #[test]
    fn test_process_extension_config_ep_config_0_ok() {
        let mut processor = Mpeg4AacProcessor::new();
        // ep_config = 0 (binary 00): needs 2 bits
        let data = BytesMut::from(&[0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.process_extension_config();
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_extension_config_ep_config_1_ok() {
        let mut processor = Mpeg4AacProcessor::new();
        // ep_config = 1 (binary 01): 0x40 = 01 000000
        let data = BytesMut::from(&[0x40][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.process_extension_config();
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_extension_config_ep_config_2_error() {
        let mut processor = Mpeg4AacProcessor::new();
        // ep_config = 2 (binary 10): 0x80 = 10 000000
        let data = BytesMut::from(&[0x80][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.process_extension_config();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().value,
            MpegErrorValue::ShouldNotComeHere
        ));
    }

    #[test]
    fn test_process_extension_config_ep_config_3_error() {
        let mut processor = Mpeg4AacProcessor::new();
        // ep_config = 3 (binary 11): 0xC0 = 11 000000
        let data = BytesMut::from(&[0xC0][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.process_extension_config();
        assert!(result.is_err());
    }

    // ========== get_audio_object_type Extended Type Tests ==========

    #[test]
    fn test_get_audio_object_type_extended_type_31() {
        let mut processor = Mpeg4AacProcessor::new();
        // Object type 31+ triggers extended encoding:
        // First 5 bits = 11111 (31), then 6 more bits for (type - 32)
        // For type 32: extended bits = 000000
        // 0xF8 = 11111 000 -> first 5 bits = 31
        // 0x00 = 00 000000 -> next 6 bits = 0, so type = 32 + 0 = 32
        let data = BytesMut::from(&[0xF8, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let object_type = processor.get_audio_object_type().unwrap();
        assert_eq!(object_type, 32);
    }

    #[test]
    fn test_get_audio_object_type_extended_type_33() {
        let mut processor = Mpeg4AacProcessor::new();
        // For type 33: first 5 bits = 11111 (31), then 6 bits = 000001 (1)
        // BitsReader reads MSB-first from each byte:
        // Byte 0: 11111_000 = 0xF8 -> reads 5 bits: 11111 (31)
        // Remaining 3 bits in byte 0: 000
        // Byte 1: need 3 more bits -> 001_xxxxx = 0x20
        // So 6-bit value = 000_001 = 1, type = 32 + 1 = 33
        let data = BytesMut::from(&[0xF8, 0x20][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let object_type = processor.get_audio_object_type().unwrap();
        assert_eq!(object_type, 33);
    }

    // ========== get_sampling_frequency with 0x0F Index Tests ==========

    #[test]
    fn test_get_sampling_frequency_explicit_value() {
        let mut processor = Mpeg4AacProcessor::new();
        // When sampling_frequency_index == 0x0F, next 24 bits are the actual frequency.
        // BitsReader reads MSB-first from byte boundary.
        // 4 bits index: 1111 (0x0F)
        // 24 bits freq: we'll use 3000 (0x000BB8) for simplicity
        // 3000 = 0000 0000 0000 1011 1011 1000
        // Stream bits: 1111 | 0000 0000 0000 1011 1011 1000
        // Byte layout: 1111_0000 | 0000_0000 | 1011_1011 | 1000_xxxx
        //              0xF0        0x00        0xBB        0x80
        let data = BytesMut::from(&[0xF0, 0x00, 0xBB, 0x80][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let freq = processor.get_sampling_frequency().unwrap();
        assert_eq!(freq, 3000);
    }

    // ========== handle_sbr_ps_extension Tests ==========

    #[test]
    fn test_handle_sbr_ps_extension_not_type5_returns_ok() {
        let mut processor = Mpeg4AacProcessor::new();
        let data = BytesMut::from(&[0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let mut ext_type: u8 = 0; // not 5
        let mut ext_freq: u32 = 0;
        let result = processor.handle_sbr_ps_extension(&mut ext_type, &mut ext_freq);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_sbr_ps_extension_type5_insufficient_bits() {
        let mut processor = Mpeg4AacProcessor::new();
        // Only 8 bits available, but needs 16
        let data = BytesMut::from(&[0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let mut ext_type: u8 = 5;
        let mut ext_freq: u32 = 0;
        let result = processor.handle_sbr_ps_extension(&mut ext_type, &mut ext_freq);
        assert!(result.is_ok()); // returns early due to insufficient bits
    }

    // ========== handle_extension_flag_type22 Tests ==========

    #[test]
    fn test_handle_extension_flag_type22() {
        let mut processor = Mpeg4AacProcessor::new();
        // Needs 5 + 11 = 16 bits
        let data = BytesMut::from(&[0x00, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.handle_extension_flag_type22();
        assert!(result.is_ok());
    }

    // ========== handle_extension_flag_type_17_19_20_23 Tests ==========

    #[test]
    fn test_handle_extension_flag_type_17_19_20_23() {
        let mut processor = Mpeg4AacProcessor::new();
        // Needs 1 + 1 + 1 = 3 bits
        let data = BytesMut::from(&[0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.handle_extension_flag_type_17_19_20_23();
        assert!(result.is_ok());
    }

    // ========== audio_specific_config_load Error Tests ==========

    #[test]
    fn test_audio_specific_config_load_empty_data() {
        let mut processor = Mpeg4AacProcessor::new();
        let result = processor.audio_specific_config_load();
        assert!(result.is_err());
    }

    #[test]
    fn test_audio_specific_config_load_invalid_freq_index() {
        let mut processor = Mpeg4AacProcessor::new();
        // Construct bytes where sampling_frequency_index = 15 (out of range)
        // byte_0 = object_type(5 bits) | freq_hi(3 bits)
        // byte_1 = freq_lo(1 bit) | channel(4 bits) | padding(3 bits)
        // For freq_index=15: 1111 -> hi=111, lo=1
        // object_type=2: 00010
        // byte_0 = 00010 111 = 0x17
        // byte_1 = 1 0010 000 = 0x90
        let data = BytesMut::from(&[0x17, 0x90][..]);
        processor.extend_data(data);
        let result = processor.audio_specific_config_load();
        assert!(result.is_err());
    }

    // ========== load_specific_config_by_type Tests ==========

    #[test]
    fn test_load_specific_config_by_type_unsupported_type() {
        let mut processor = Mpeg4AacProcessor::new();
        processor.mpeg4_aac.object_type = 30; // Not in 1..=7, 17, 19..=23, or 8
        let data = BytesMut::from(&[0x00, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.load_specific_config_by_type();
        assert!(result.is_ok()); // Falls through to Ok(())
    }

    // ========== celp_specific_config_load Tests ==========

    #[test]
    fn test_celp_specific_config_load_first_bit_1_excitation_mode_1() {
        let mut processor = Mpeg4AacProcessor::new();
        // First bit = 1, excitation_mode = 1
        // Reads: 1 bit, 1 bit (excitation_mode=1), 1 bit, 1 bit, then 3 bits
        // Total: 1 + 1 + 1 + 1 + 3 = 7 bits
        // Bits: 1 1 x x x x x x (where x are the remaining bits)
        // Let's use: 1100 0000 0000 0000 = 0xC0 0x00
        let data = BytesMut::from(&[0xC0, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.celp_specific_config_load();
        assert!(result.is_ok());
    }

    #[test]
    fn test_celp_specific_config_load_first_bit_1_excitation_mode_0() {
        let mut processor = Mpeg4AacProcessor::new();
        // First bit = 1, excitation_mode = 0
        // Reads: 1 bit, 1 bit (excitation_mode=0), 1 bit, 1 bit, then 5 + 2 + 1 = 8 bits
        // Total: 1 + 1 + 1 + 1 + 5 + 2 + 1 = 12 bits
        // Bits: 1 0 x x xxxx xx x xxxxx
        // Let's use: 1000 0000 0000 0000 = 0x80 0x00
        let data = BytesMut::from(&[0x80, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.celp_specific_config_load();
        assert!(result.is_ok());
    }

    #[test]
    fn test_celp_specific_config_load_first_bit_0() {
        let mut processor = Mpeg4AacProcessor::new();
        // First bit = 0
        // Reads: 1 bit, then 1 bit, then 2 bits
        // Total: 1 + 1 + 2 = 4 bits
        // Bits: 0 x xx xxxxx
        // Let's use: 0000 0000 = 0x00
        let data = BytesMut::from(&[0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.celp_specific_config_load();
        assert!(result.is_ok());
    }

    // ========== ga_specific_config_load Tests ==========

    #[test]
    fn test_ga_specific_config_load_basic() {
        let mut processor = Mpeg4AacProcessor::new();
        // Basic path: frameLengthFlag=0, dependsOnCoreCoder=0, extensionFlag=0
        // Reads: 1 bit (frameLengthFlag) + 1 bit (dependsOnCoreCoder) + 1 bit (extensionFlag) = 3 bits
        // channel_configuration != 0, so no pce_load
        // object_type not 6 or 20, so no layer read
        processor.mpeg4_aac.channel_configuration = 2;
        processor.mpeg4_aac.object_type = 2;
        let data = BytesMut::from(&[0x00][..]); // All bits 0
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.ga_specific_config_load();
        assert!(result.is_ok());
    }

    #[test]
    fn test_ga_specific_config_load_depends_on_core_coder() {
        let mut processor = Mpeg4AacProcessor::new();
        // dependsOnCoreCoder = 1 (reads 14 additional bits)
        // Reads: 1 bit (frameLengthFlag) + 1 bit (dependsOnCoreCoder=1) + 14 bits + 1 bit (extensionFlag)
        // Bits: 0 1 (14 bits) 0
        // Total: 17 bits
        // Bit layout: 01 | 00000000 000000 | 0
        //             01000000 00000000 0x
        //             0x40     0x00     0x00
        processor.mpeg4_aac.channel_configuration = 2;
        processor.mpeg4_aac.object_type = 2;
        let data = BytesMut::from(&[0x40, 0x00, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.ga_specific_config_load();
        assert!(result.is_ok());
    }

    #[test]
    fn test_ga_specific_config_load_extension_flag_type22() {
        let mut processor = Mpeg4AacProcessor::new();
        // extensionFlag = 1, object_type = 22
        // Reads: 1 + 1 + 1 (ext_flag=1) + 5 + 11 (type22) + 1 (final bit) = 20 bits
        processor.mpeg4_aac.channel_configuration = 2;
        processor.mpeg4_aac.object_type = 22;
        // Bits: 0 0 1 | (5 bits) (11 bits) 0
        // Let's use: 001 00000 00000000000 0
        //            00100000 00000000 000x xxxx
        //            0x20     0x00     0x00
        let data = BytesMut::from(&[0x20, 0x00, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.ga_specific_config_load();
        assert!(result.is_ok());
    }

    #[test]
    fn test_ga_specific_config_load_extension_flag_type17() {
        let mut processor = Mpeg4AacProcessor::new();
        // extensionFlag = 1, object_type = 17
        // Reads: 1 + 1 + 1 (ext_flag=1) + 1 + 1 + 1 (type17) + 1 (final bit) = 8 bits
        processor.mpeg4_aac.channel_configuration = 2;
        processor.mpeg4_aac.object_type = 17;
        // Bits: 0 0 1 xxx 0
        // Let's use: 00100000 = 0x20
        let data = BytesMut::from(&[0x20][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.ga_specific_config_load();
        assert!(result.is_ok());
    }

    #[test]
    fn test_ga_specific_config_load_channel_config_0_triggers_pce() {
        let mut processor = Mpeg4AacProcessor::new();
        processor.mpeg4_aac.channel_configuration = 0; // Triggers pce_load
        processor.mpeg4_aac.object_type = 2;

        // Build minimal PCE data: 4+2+4+4+4+4+2+3+4 = 31 bits for header
        // Then 3 optional flags (3 bits), alignment, comment field length (8 bits)
        // Total minimum: ~50 bits = 7 bytes
        // PCE structure:
        // element_instance_tag(4) object_type(2) sampling_freq_idx(4)
        // num_front(4) num_side(4) num_back(4) num_lfe(2) num_assoc(3) num_valid_cc(4)
        // 3 optional flags (1 bit each)
        // alignment to byte boundary
        // comment_field_bytes(8)

        // Let's create minimal PCE: all counts = 0, no optional elements, no comment
        // Bits: eeee tt ffff FFFF SSSS BBBB ll aaa VVVV | 0 0 0 | padding | 00000000
        // Using: 0000 00 0000 0000 0000 0000 00 000 0000 = 0x00 0x00 0x00 0x00
        //        0 0 0 (padding to byte) = need 5 more bits for alignment after bit 34
        //        00000000 (comment length)
        // Total bits: 31 + 3 = 34 bits, alignment adds 6 bits -> 40 bits, +8 for comment = 48 bits = 6 bytes

        // But we need ga_specific_config_load bits first: 1 + 1 + 1 = 3 bits
        // So total: 3 + 48 = 51 bits = 7 bytes
        // ga bits: 001 (no frame length flag, no depends, ext flag=0)
        // Combined: 001 + PCE bits
        let data = BytesMut::from(&[0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.ga_specific_config_load();
        assert!(result.is_ok());
    }

    #[test]
    fn test_ga_specific_config_load_object_type_6_reads_layer() {
        let mut processor = Mpeg4AacProcessor::new();
        processor.mpeg4_aac.channel_configuration = 2;
        processor.mpeg4_aac.object_type = 6; // Triggers layerNr read (3 bits)
        // Reads: 1 + 1 + 1 (flags) + 3 (layerNr) = 6 bits
        // Bits: 000 xxx
        // Let's use: 00000000 = 0x00
        let data = BytesMut::from(&[0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.ga_specific_config_load();
        assert!(result.is_ok());
    }

    #[test]
    fn test_ga_specific_config_load_object_type_20_reads_layer() {
        let mut processor = Mpeg4AacProcessor::new();
        processor.mpeg4_aac.channel_configuration = 2;
        processor.mpeg4_aac.object_type = 20; // Also triggers layerNr read
        let data = BytesMut::from(&[0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );
        let result = processor.ga_specific_config_load();
        assert!(result.is_ok());
    }

    // ========== pce_load Tests ==========

    #[test]
    fn test_pce_load_minimal_no_elements() {
        let mut processor = Mpeg4AacProcessor::new();
        processor.mpeg4_aac.ps = 0;

        // PCE structure with all element counts = 0:
        // element_instance_tag(4) = 0000
        // object_type(2) = 00
        // sampling_frequency_index(4) = 0000
        // num_front_channel_elements(4) = 0000
        // num_side_channel_elements(4) = 0000
        // num_back_channel_elements(4) = 0000
        // num_lfe_channel_elements(2) = 00
        // num_assoc_data_elements(3) = 000
        // num_valid_cc_elements(4) = 0000
        // Total: 31 bits

        // Then 3 optional flags (mono, stereo, matrix):
        // 0 0 0 = 3 bits
        // Total so far: 34 bits

        // Then alignment to 8-bit boundary: 34 bits needs 6 more bits to reach 40 (5 bytes)
        // After alignment, comment_field_bytes(8) = 00000000
        // Total: 48 bits = 6 bytes

        // Bit layout:
        // Byte 0: 0000 0000 = 0x00 (element_instance_tag + object_type)
        // Byte 1: 0000 0000 = 0x00 (sampling_freq_idx + num_front)
        // Byte 2: 0000 0000 = 0x00 (num_side + num_back)
        // Byte 3: 0000 0000 = 0x00 (num_lfe + num_assoc + num_valid_cc)
        // Byte 4: 00 000000 = 0x00 (3 optional flags + 5 padding bits for alignment)
        // Byte 5: 0000 0000 = 0x00 (comment_field_bytes = 0)

        let data = BytesMut::from(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        let result = processor.pce_load();
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_aac.channels, 0); // No channels
    }

    #[test]
    fn test_pce_load_with_front_channel_mono() {
        let mut processor = Mpeg4AacProcessor::new();
        processor.mpeg4_aac.ps = 0;

        // num_front_channel_elements = 1
        // Each front element: cpe(1 bit) + element_instance_tag(4 bits) = 5 bits
        // Let's set cpe = 0 (mono), so it adds 1 channel

        // PCE header bit layout (31 bits):
        // Bits 0-3: element_instance_tag = 0000
        // Bits 4-5: object_type = 00
        // Bits 6-9: sampling_frequency_index = 0000
        // Bits 10-13: num_front_channel_elements = 0001
        // Bits 14-17: num_side_channel_elements = 0000
        // Bits 18-21: num_back_channel_elements = 0000
        // Bits 22-23: num_lfe_channel_elements = 00
        // Bits 24-26: num_assoc_data_elements = 000
        // Bits 27-30: num_valid_cc_elements = 0000

        // BitsReader reads MSB-first from each byte:
        // Byte 0 (bits 0-7): 0000_0000 = 0x00
        // Byte 1 (bits 8-15): 00_0001_00 = 0x04 (num_front=1 in bits 10-13)
        // Byte 2 (bits 16-23): 0000_0000 = 0x00
        // Byte 3 (bits 24-31): 000_0000_0 = 0x00 (bit 31 is first optional flag)

        // After 31 bits, we read 3 optional flags (all 0): 3 more bits
        // Then 1 front element: cpe(1)=0 + tag(4)=0000 = 5 bits
        // Total after header: 31 + 3 + 5 = 39 bits

        // Byte 4 (bits 32-39): opt_flag2(1)=0 + opt_flag3(1)=0 + cpe(1)=0 + tag(4)=0000 + align(1)=0
        //                      0000_0000 = 0x00
        // Alignment adds 1 bit to reach bit 40 (byte boundary)
        // Byte 5 (bits 40-47): comment_field_bytes(8) = 0x00

        let data = BytesMut::from(&[0x00, 0x04, 0x00, 0x00, 0x00, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        let result = processor.pce_load();
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_aac.channels, 1); // 1 mono channel
    }

    #[test]
    fn test_pce_load_with_front_channel_stereo() {
        let mut processor = Mpeg4AacProcessor::new();
        processor.mpeg4_aac.ps = 0;

        // num_front_channel_elements = 1, cpe = 1 (stereo)
        // Should add 2 channels

        // Same layout as mono test, but with cpe=1
        // Byte 0: 0x00
        // Byte 1: 0x04 (num_front=1 in bits 10-13)
        // Byte 2: 0x00
        // Byte 3: 0x00 (includes first optional flag = 0 in bit 31)
        // Byte 4: bit 32-39: opt_flag2(1)=0, opt_flag3(1)=0, cpe(1)=1, tag(4)=0000, align(1)=0
        //         00_1_0000_0 = 0x20
        // Byte 5: 0x00 (comment_field_bytes=0)

        let data = BytesMut::from(&[0x00, 0x04, 0x00, 0x00, 0x20, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        let result = processor.pce_load();
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_aac.channels, 2); // 1 stereo pair = 2 channels
    }

    #[test]
    fn test_pce_load_with_lfe_element() {
        let mut processor = Mpeg4AacProcessor::new();
        processor.mpeg4_aac.ps = 0;

        // num_lfe_channel_elements = 1
        // Each LFE element: element_instance_tag(4 bits), adds 1 channel

        // Using byte-aligned layout:
        // Byte 0: element_instance_tag(4) + object_type(2) + sampling_freq_idx(2 upper)
        //         0000_0000 = 0x00
        // Byte 1: sampling_freq_idx(2 lower) + num_front(4) + num_side(2 upper)
        //         0000_0000 = 0x00
        // Byte 2: num_side(2 lower) + num_back(4) + num_lfe(2)
        //         0000_0001 = 0x01 (num_lfe=01 in bits 6-7)
        // Byte 3: num_assoc(3) + num_valid_cc(4) + opt_flag_1(1)
        //         0000_0000 = 0x00
        // Byte 4: opt_flag_2(1) + opt_flag_3(1) + lfe_tag(4) + align(2)
        //         0000_0000 = 0x00
        // Byte 5: comment_field_bytes(8)
        //         0000_0000 = 0x00

        let data = BytesMut::from(&[0x00, 0x00, 0x01, 0x00, 0x00, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        let result = processor.pce_load();
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_aac.channels, 1); // 1 LFE channel
    }

    #[test]
    fn test_pce_load_with_comment_field() {
        let mut processor = Mpeg4AacProcessor::new();
        processor.mpeg4_aac.ps = 0;

        // Minimal PCE with comment_field_bytes = 2
        // Using byte-aligned layout with all element counts = 0:
        // Byte 0: 0x00
        // Byte 1: 0x00
        // Byte 2: 0x00
        // Byte 3: 0x00 (includes opt_flag_1=0 in bit 31)
        // Byte 4: opt_flag_2(1)=0, opt_flag_3(1)=0, align(6)=0 = 0x00
        // Byte 5: comment_field_bytes(8) = 0x02
        // Byte 6: comment byte 1 = 0xAB
        // Byte 7: comment byte 2 = 0xCD

        let data = BytesMut::from(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xAB, 0xCD][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        let result = processor.pce_load();
        assert!(result.is_ok());
    }

    // ========== audio_specific_config_load2 Branch Tests ==========

    #[test]
    fn test_audio_specific_config_load2_object_type_5_sbr() {
        let mut processor = Mpeg4AacProcessor::new();

        // object_type = 5 (SBR), should set sbr = 1
        // Layout:
        // object_type(5 bits) = 00101 (5)
        // sampling_freq_index(4 bits) = 0011 (3 = 48000Hz)
        // channel_config(4 bits) = 0010 (2 = stereo)
        // extension_sampling_freq_index(4 bits) = 0011
        // new object_type(5 bits) = 00010 (2 = AAC-LC)
        // frameLengthFlag(1) = 0
        // dependsOnCoreCoder(1) = 0
        // extensionFlag(1) = 0

        // Bits: 00101 0011 0010 0011 00010 0 0 0
        // Bytes: 00101001 10010001 10001000 0
        //        0x29     0x91     0x88     0x00

        let data = BytesMut::from(&[0x29, 0x91, 0x88, 0x00][..]);
        processor.bytes_reader = crate::io::bytes_reader::BytesReader::new(data);

        let result = processor.audio_specific_config_load2();
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_aac.object_type, 2); // Changed to AAC-LC
        assert_eq!(processor.mpeg4_aac.sbr, 1); // SBR enabled
        assert_eq!(processor.mpeg4_aac.ps, 0); // PS not enabled
    }

    #[test]
    fn test_audio_specific_config_load2_object_type_29_ps_sbr() {
        let mut processor = Mpeg4AacProcessor::new();

        // object_type = 29 (PS+SBR), should set sbr = 1 and ps = 1
        // Layout:
        // object_type(5 bits) = 11101 (29)
        // sampling_freq_index(4 bits) = 0011 (3 = 48000Hz)
        // channel_config(4 bits) = 0010 (2 = stereo)
        // extension_sampling_freq_index(4 bits) = 0011
        // new object_type(5 bits) = 00010 (2 = AAC-LC)
        // frameLengthFlag(1) = 0
        // dependsOnCoreCoder(1) = 0
        // extensionFlag(1) = 0

        // Bits: 11101 0011 0010 0011 00010 0 0 0
        // Bytes: 11101001 10010001 10001000 0
        //        0xE9     0x91     0x88     0x00

        let data = BytesMut::from(&[0xE9, 0x91, 0x88, 0x00][..]);
        processor.bytes_reader = crate::io::bytes_reader::BytesReader::new(data);

        let result = processor.audio_specific_config_load2();
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_aac.object_type, 2);
        assert_eq!(processor.mpeg4_aac.sbr, 1);
        assert_eq!(processor.mpeg4_aac.ps, 1); // PS enabled for type 29
    }

    #[test]
    fn test_audio_specific_config_load2_object_type_17_process_extension() {
        let mut processor = Mpeg4AacProcessor::new();

        // object_type = 17 triggers process_extension_config
        // Layout:
        // object_type(5 bits) = 10001 (17)
        // sampling_freq_index(4 bits) = 0011 (3 = 48000Hz)
        // channel_config(4 bits) = 0010 (2 = stereo)
        // frameLengthFlag(1) = 0
        // dependsOnCoreCoder(1) = 0
        // extensionFlag(1) = 0
        // ep_config(2 bits) = 00 (from process_extension_config)

        // Bits: 10001 0011 0010 0 0 0 00
        // Bytes: 10001001 10010000 0
        //        0x89     0x90     0x00

        let data = BytesMut::from(&[0x89, 0x90, 0x00][..]);
        processor.bytes_reader = crate::io::bytes_reader::BytesReader::new(data);

        let result = processor.audio_specific_config_load2();
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_aac.object_type, 17);
    }

    #[test]
    fn test_audio_specific_config_load2_handle_sbr_extension_with_ps() {
        let mut processor = Mpeg4AacProcessor::new();

        // AAC-LC that later has SBR extension in handle_sbr_ps_extension
        // To trigger handle_sbr_ps_extension with type 5, we need:
        // - Initial object_type != 5 or 29
        // - Then have enough bits for handle_sbr_ps_extension to read

        // Layout:
        // object_type(5 bits) = 00010 (2 = AAC-LC)
        // sampling_freq_index(4 bits) = 0011 (3 = 48000Hz)
        // channel_config(4 bits) = 0010 (2 = stereo)
        // frameLengthFlag(1) = 0
        // dependsOnCoreCoder(1) = 0
        // extensionFlag(1) = 1 (to have handle_sbr_ps_extension later, but it needs extension_audio_object_type=5)

        // Actually, handle_sbr_ps_extension is only called if extension_audio_object_type is already 5,
        // which happens when object_type is 5 or 29. Otherwise it's 0 and skips.
        // Let me create a test for when it does trigger with sufficient bits

        // For this, I need object_type = 5 (set extension_audio_object_type = 5)
        // Then bits_reader.len() >= 16
        // sync_extension_type = SYNC_EXTENSION_TYPE_SBR (0x2B7 = 11 bits)
        // Then get_audio_object_type returns 5
        // Then handle_sbr_extension sets sbr bit and checks for PS

        // Layout:
        // object_type(5 bits) = 00101 (5)
        // sampling_freq_index(4 bits) = 0011
        // channel_config(4 bits) = 0010
        // extension_sampling_freq_index(4 bits) = 0011
        // new object_type(5 bits) = 00010 (2)
        // frameLengthFlag(1) = 0
        // dependsOnCoreCoder(1) = 0
        // extensionFlag(1) = 0
        // (now in handle_sbr_ps_extension)
        // sync_extension_type(11 bits) = 010 1011 0111 (0x2B7)
        // extension_audio_object_type(5 bits) = 00101 (5)
        // sbr(1 bit) = 1
        // extension_sampling_freq_index2(4 bits) = 0011
        // sync_extension_type_ps(11 bits) = 101 0100 1000 (0x548)
        // ps(1 bit) = 1

        // Bits: 00101 0011 0010 0011 00010 0 0 0 | 010 1011 0111 00101 1 0011 101 0100 1000 1
        // Let me break this down byte by byte:
        // 00101001 10010001 10001000 00101011 01110010 11001110 10100100 01
        // 0x29     0x91     0x88     0x2B     0x72     0xCE     0xA4     0x40

        let data = BytesMut::from(&[0x29, 0x91, 0x88, 0x2B, 0x72, 0xCE, 0xA4, 0x40][..]);
        processor.bytes_reader = crate::io::bytes_reader::BytesReader::new(data);

        let result = processor.audio_specific_config_load2();
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_aac.sbr, 1);
        assert_eq!(processor.mpeg4_aac.ps, 1); // PS should be set
    }

    #[test]
    fn test_audio_specific_config_load2_explicit_sampling_frequency() {
        let mut processor = Mpeg4AacProcessor::new();

        // sampling_frequency_index = 0x0F triggers explicit 24-bit frequency read
        // Layout:
        // object_type(5 bits) = 00010 (2)
        // sampling_freq_index(4 bits) = 1111 (0x0F)
        // sampling_frequency(24 bits) = 48000 = 0x00BB80
        // channel_config(4 bits) = 0010
        // frameLengthFlag(1) = 0
        // dependsOnCoreCoder(1) = 0
        // extensionFlag(1) = 0

        // Bits: 00010 1111 | 00000000 10111011 10000000 | 0010 0 0 0
        // Bytes: 00010111 10000000 01011101 11000000 00100000
        //        0x17     0x80     0x5D     0xC0     0x20

        let data = BytesMut::from(&[0x17, 0x80, 0x5D, 0xC0, 0x20][..]);
        processor.bytes_reader = crate::io::bytes_reader::BytesReader::new(data);

        let result = processor.audio_specific_config_load2();
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_aac.sampling_frequency_index, 0x0F);
        assert_eq!(processor.mpeg4_aac.sampling_frequency, 48000);
    }

    // ========== handle_sbr_extension Tests ==========

    #[test]
    fn test_handle_sbr_extension_sbr_bit_set_with_ps() {
        let mut processor = Mpeg4AacProcessor::new();

        // sbr bit = 1, sufficient bits for PS check
        // Layout:
        // sbr(1 bit) = 1
        // extension_sampling_freq_index(4 bits) = 0011
        // sync_extension_type_ps(11 bits) = 101 0100 1000 (0x548)
        // ps(1 bit) = 1

        // Bits: 1 0011 10101001000 1
        // Bytes: 10011101 01001000 1
        //        0x9D     0x48     0x80

        let data = BytesMut::from(&[0x9D, 0x48, 0x80][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        let mut ext_freq: u32 = 0;
        let result = processor.handle_sbr_extension(&mut ext_freq);
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_aac.sbr, 1);
        assert_eq!(processor.mpeg4_aac.ps, 1);
    }

    #[test]
    fn test_handle_sbr_extension_sbr_bit_set_no_ps() {
        let mut processor = Mpeg4AacProcessor::new();

        // sbr bit = 1, but sync_extension_type != SYNC_EXTENSION_TYPE_PS
        // Layout:
        // sbr(1 bit) = 1
        // extension_sampling_freq_index(4 bits) = 0011
        // sync_extension_type(11 bits) = 000 0000 0000 (not 0x548)

        // Bits: 1 0011 00000000000
        // Bytes: 10011000 00000000
        //        0x98     0x00

        let data = BytesMut::from(&[0x98, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        let mut ext_freq: u32 = 0;
        let result = processor.handle_sbr_extension(&mut ext_freq);
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_aac.sbr, 1);
        assert_eq!(processor.mpeg4_aac.ps, 0); // PS not set
    }

    #[test]
    fn test_handle_sbr_extension_sbr_bit_not_set() {
        let mut processor = Mpeg4AacProcessor::new();

        // sbr bit = 0, should return early
        let data = BytesMut::from(&[0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        let mut ext_freq: u32 = 0;
        let result = processor.handle_sbr_extension(&mut ext_freq);
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_aac.sbr, 0);
    }

    // ========== handle_sbr_extension_type22 Tests ==========

    #[test]
    fn test_handle_sbr_extension_type22_sbr_bit_set() {
        let mut processor = Mpeg4AacProcessor::new();

        // sbr bit = 1, reads extension_sampling_freq_index (4 bits) then reads 4 more bits
        // Layout:
        // sbr(1 bit) = 1
        // extension_sampling_freq_index(4 bits) = 0011
        // reserved(4 bits) = 0000

        // Bits: 1 0011 0000
        // Bytes: 10011000 0
        //        0x98     0x00

        let data = BytesMut::from(&[0x98, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        let mut ext_freq: u32 = 0;
        let result = processor.handle_sbr_extension_type22(&mut ext_freq);
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_aac.sbr, 1);
    }

    #[test]
    fn test_handle_sbr_extension_type22_sbr_bit_not_set() {
        let mut processor = Mpeg4AacProcessor::new();

        // sbr bit = 0, still reads extension_sampling_freq and 4 bits
        // Layout:
        // sbr(1 bit) = 0
        // extension_sampling_freq_index(4 bits) = 0011
        // reserved(4 bits) = 0000

        let data = BytesMut::from(&[0x18, 0x00][..]);
        processor.bits_reader = crate::io::bits_reader::BitsReader::new(
            crate::io::bytes_reader::BytesReader::new(data),
        );

        let mut ext_freq: u32 = 0;
        let result = processor.handle_sbr_extension_type22(&mut ext_freq);
        assert!(result.is_ok());
        assert_eq!(processor.mpeg4_aac.sbr, 0);
    }
}
