use {
    super::errors::{MpegAacError, MpegErrorValue},
    crate::bytesio::{
        bits_reader::BitsReader, bits_writer::BitsWriter, bytes_reader::BytesReader,
        bytes_writer::BytesWriter,
    },
    bytes::BytesMut,
};

const AAC_FREQUENCE_SIZE: usize = 13;
const AAC_FREQUENCE: [u32; AAC_FREQUENCE_SIZE] = [
    96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350,
];

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
        //11 88 56 E5
        let byte_0 = self.bytes_reader.read_u8()?;
        self.mpeg4_aac.object_type = (byte_0 >> 3) & 0x1F;

        let byte_1 = self.bytes_reader.read_u8()?;
        self.mpeg4_aac.sampling_frequency_index = ((byte_0 & 0x07) << 1) | ((byte_1 >> 7) & 0x01);
        self.mpeg4_aac.channel_configuration = (byte_1 >> 3) & 0x0F;
        self.mpeg4_aac.channels = self.mpeg4_aac.channel_configuration;
        self.mpeg4_aac.sampling_frequency =
            AAC_FREQUENCE[self.mpeg4_aac.sampling_frequency_index as usize];

        // log::info!("aac info: {:?}", self.mpeg4_aac);

        // if self.bytes_reader.len() > 2 {
        //return self.audio_specific_config_load2();
        // }

        // self.bytes_reader.read_u8()?;
        // self.bytes_reader.read_u8()?;

        self.bytes_reader.extract_remaining_bytes();

        Ok(self)
    }

    pub fn audio_specific_config_load2(&mut self) -> Result<(), MpegAacError> {
        let remain_bytes = self.bytes_reader.extract_remaining_bytes();
        // self.bits_reader.extend_from_bytesmut(remain_bytes);
        self.bits_reader.extend_data(remain_bytes);

        self.mpeg4_aac.object_type = self.get_audio_object_type()?;
        self.mpeg4_aac.sampling_frequency_index = self.get_sampling_frequency()?;
        self.mpeg4_aac.channel_configuration = self.bits_reader.read_n_bits(4)? as u8;

        let mut extension_audio_object_type: u8;
        let mut extension_sampling_frequency_index: u8 = 0;
        let mut extension_channel_configuration: u8 = 0;

        if self.mpeg4_aac.object_type == 5 || self.mpeg4_aac.object_type == 29 {
            extension_audio_object_type = 5;
            self.mpeg4_aac.sbr = 1;
            {
                if self.mpeg4_aac.object_type == 29 {
                    self.mpeg4_aac.ps = 1;
                }
                extension_sampling_frequency_index = self.get_sampling_frequency()?;
                self.mpeg4_aac.object_type = self.get_audio_object_type()?;

                if self.mpeg4_aac.object_type == 22 {
                    extension_channel_configuration = self.bits_reader.read_n_bits(4)? as u8;
                }
            }
        } else {
            extension_audio_object_type = 0;
        }

        let ep_config: u64;

        match self.mpeg4_aac.object_type {
            1 | 2 | 3 | 4 | 5 | 6 | 7 | 17 | 19 | 20 | 21 | 22 | 23 => {
                self.ga_specific_config_load()?;
            }
            8 => {
                self.celp_specific_config_load()?;
            }
            _ => {}
        }

        match self.mpeg4_aac.object_type {
            17 | 19 | 20 | 21 | 22 | 23 | 24 | 25 | 26 | 27 | 39 => {
                ep_config = self.bits_reader.read_n_bits(2)?;

                match ep_config {
                    2 | 3 => {
                        return Err(MpegAacError {
                            value: MpegErrorValue::ShouldNotComeHere,
                        });
                    }

                    _ => {}
                }
            }
            _ => {}
        }

        let mut sync_extension_type: u64;

        if 5 != extension_audio_object_type && self.bits_reader.len() >= 16 {
            sync_extension_type = self.bits_reader.read_n_bits(11)?;

            if 0x2B7 == sync_extension_type {
                extension_audio_object_type = self.get_audio_object_type()?;

                match extension_audio_object_type {
                    5 => {
                        self.mpeg4_aac.sbr = self.bits_reader.read_n_bits(1)? as usize;
                        if self.mpeg4_aac.sbr > 0 {
                            extension_sampling_frequency_index = self.get_sampling_frequency()?;
                            if self.bits_reader.len() >= 12 {
                                sync_extension_type = self.bits_reader.read_n_bits(11)?;
                                if 0x548 == sync_extension_type {
                                    self.mpeg4_aac.ps = self.bits_reader.read_n_bits(1)? as usize;
                                }
                            }
                        }
                    }
                    22 => {
                        self.mpeg4_aac.sbr = self.bits_reader.read_n_bits(1)? as usize;

                        if self.mpeg4_aac.sbr > 0 {
                            extension_sampling_frequency_index = self.get_sampling_frequency()?;
                        }

                        extension_channel_configuration = self.bits_reader.read_n_bits(4)? as u8;
                    }

                    _ => {}
                }
            }
        }

        self.bits_reader.bits_aligment_8();

        log::trace!(
            "remove warnings: {} {} {}",
            extension_audio_object_type,
            extension_sampling_frequency_index,
            extension_channel_configuration
        );

        Ok(())
    }

    pub fn celp_specific_config_load(&mut self) -> Result<(), MpegAacError> {
        let excitation_mode: u64;

        if self.bits_reader.read_n_bits(1)? > 0 {
            excitation_mode = self.bits_reader.read_n_bits(1)?;
            self.bits_reader.read_n_bits(1)?;
            self.bits_reader.read_n_bits(1)?;

            if excitation_mode == 1 {
                self.bits_reader.read_n_bits(3)?;
            } else if excitation_mode == 0 {
                self.bits_reader.read_n_bits(5)?;
                self.bits_reader.read_n_bits(2)?;
                self.bits_reader.read_n_bits(1)?;
            }
        } else {
            self.bits_reader.read_n_bits(1)?;
            self.bits_reader.read_n_bits(2)?;
        }

        // if self.bits_reader.read_n_bits(1)? > 0 {
        //     self.bits_reader.read_n_bits(2)?;
        // } else {
        //     self.bits_reader.read_n_bits(2)?;
        // }

        Ok(())
    }
    pub fn ga_specific_config_load(&mut self) -> Result<(), MpegAacError> {
        self.bits_reader.read_n_bits(1)?;

        if self.bits_reader.read_n_bits(1)? > 0 {
            self.bits_reader.read_n_bits(14)?;
        }
        let extension_flag: u64 = self.bits_reader.read_n_bits(1)?;

        if 0 == self.mpeg4_aac.channel_configuration {
            self.pce_load()?;
        }

        if self.mpeg4_aac.object_type == 6 || self.mpeg4_aac.object_type == 20 {
            self.bits_reader.read_n_bits(3)?;
        }

        if extension_flag > 0 {
            match self.mpeg4_aac.object_type {
                22 => {
                    self.bits_reader.read_n_bits(5)?;
                    self.bits_reader.read_n_bits(11)?;
                }
                17 | 19 | 20 | 23 => {
                    self.bits_reader.read_n_bits(1)?;
                    self.bits_reader.read_n_bits(1)?;
                    self.bits_reader.read_n_bits(1)?;
                }
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

    pub fn pce_load(&mut self) -> Result<u8, MpegAacError> {
        let mut cpe: u64 = 0;
        let mut tag: u64 = 0;

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
            cpe = self.mpeg4_bits_copy(&mut pce_bits_vec, 1)?;
            tag = self.mpeg4_bits_copy(&mut pce_bits_vec, 4)?;

            if cpe > 0 || self.mpeg4_aac.ps > 0 {
                self.mpeg4_aac.channels += 2;
            } else {
                self.mpeg4_aac.channels += 1;
            }
        }

        for _ in 0..num_side_channel_elements {
            cpe = self.mpeg4_bits_copy(&mut pce_bits_vec, 1)?;
            tag = self.mpeg4_bits_copy(&mut pce_bits_vec, 4)?;

            if cpe > 0 || self.mpeg4_aac.ps > 0 {
                self.mpeg4_aac.channels += 2;
            } else {
                self.mpeg4_aac.channels += 1;
            }
        }

        for _ in 0..num_back_channel_elements {
            cpe = self.mpeg4_bits_copy(&mut pce_bits_vec, 1)?;
            tag = self.mpeg4_bits_copy(&mut pce_bits_vec, 4)?;

            if cpe > 0 || self.mpeg4_aac.ps > 0 {
                self.mpeg4_aac.channels += 2;
            } else {
                self.mpeg4_aac.channels += 1;
            }
        }

        for _ in 0..num_lfe_channel_elements {
            tag = self.mpeg4_bits_copy(&mut pce_bits_vec, 4)?;
            self.mpeg4_aac.channels += 1;
        }

        for _ in 0..num_assoc_data_elements {
            tag = self.mpeg4_bits_copy(&mut pce_bits_vec, 4)?;
        }

        for _ in 0..num_valid_cc_elements {
            cpe = self.mpeg4_bits_copy(&mut pce_bits_vec, 1)?;
            tag = self.mpeg4_bits_copy(&mut pce_bits_vec, 4)?;
        }

        pce_bits_vec.bits_aligment_8()?;
        self.bits_reader.bits_aligment_8();

        let comment_field_bytes: u64 = self.mpeg4_bits_copy(&mut pce_bits_vec, 8)?;

        for _ in 0..comment_field_bytes {
            self.mpeg4_bits_copy(&mut pce_bits_vec, 8)?;
        }

        let rv = pce_bits_vec.len().div_ceil(8);

        log::trace!(
            "remove warnings: {} {} {} {} {}",
            tag,
            element_instance_tag,
            object_type,
            sampling_frequency_index,
            cpe
        );

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

    pub fn get_sampling_frequency(&mut self) -> Result<u8, MpegAacError> {
        let mut sampling_frequency_index: u64;

        sampling_frequency_index = self.bits_reader.read_n_bits(4)?;
        if sampling_frequency_index == 0x0F {
            sampling_frequency_index = self.bits_reader.read_n_bits(24)?;
        }

        Ok(sampling_frequency_index as u8)
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
        assert!(processor.bytes_reader.len() > 0);
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
        processor.bytes_reader = crate::bytesio::bytes_reader::BytesReader::new(raw_data);

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
        processor.bytes_reader = crate::bytesio::bytes_reader::BytesReader::new(raw_data);

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
        processor.bytes_reader = crate::bytesio::bytes_reader::BytesReader::new(raw_data);

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
        processor.bits_reader = crate::bytesio::bits_reader::BitsReader::new(
            crate::bytesio::bytes_reader::BytesReader::new(data),
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
        processor.bits_reader = crate::bytesio::bits_reader::BitsReader::new(
            crate::bytesio::bytes_reader::BytesReader::new(data),
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
        processor.bits_reader = crate::bytesio::bits_reader::BitsReader::new(
            crate::bytesio::bytes_reader::BytesReader::new(data),
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
        processor.bits_reader = crate::bytesio::bits_reader::BitsReader::new(
            crate::bytesio::bytes_reader::BytesReader::new(data),
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
        processor.bits_reader = crate::bytesio::bits_reader::BitsReader::new(
            crate::bytesio::bytes_reader::BytesReader::new(data),
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
            processor.bits_reader = crate::bytesio::bits_reader::BitsReader::new(
                crate::bytesio::bytes_reader::BytesReader::new(config.clone()),
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
}
