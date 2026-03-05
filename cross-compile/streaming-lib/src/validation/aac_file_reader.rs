use thiserror::Error;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};

/// Errors that can occur while reading AAC files
#[derive(Error, Debug)]
pub enum AacFileError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid ADTS header")]
    InvalidAdtsHeader,

    #[error("Invalid ADTS frame length")]
    InvalidFrameLength,

    #[error("No frames found in file")]
    NoFramesFound,

    #[error("Unsupported AAC profile: {0}")]
    UnsupportedProfile(u8),

    #[error("Unsupported sampling frequency index: {0}")]
    UnsupportedSamplingFrequency(u8),

    #[error("Invalid sync word")]
    InvalidSyncWord,
}

/// Represents a parsed AAC frame
#[derive(Debug, Clone)]
pub struct AacFrame {
    /// Raw AAC payload (WITHOUT ADTS header)
    pub data: Vec<u8>,

    /// ADTS header size for this frame (7 or 9 bytes)
    pub adts_header_size: usize,

    /// Total frame size including header
    pub total_frame_size: usize,

    /// AAC profile (1=Main, 2=LC, 3=SSR, 4=LTP)
    pub profile: u8,

    /// Sampling frequency in Hz
    pub sample_rate: u32,

    /// Channel configuration (1=mono, 2=stereo, etc.)
    pub channels: u8,
}

/// AAC ADTS file reader for streaming applications
pub struct AacFileReader {
    file: File,
    buffer: Vec<u8>,
    parse_data: Vec<u8>,
    parse_start: usize,
    parse_eof: bool,
    current_pos: u64,
    file_size: u64,

    // AAC-specific fields
    sample_rate: u32,
    frame_duration_ms: u32,

    // Cached from first frame's ADTS header
    audio_config: Option<Vec<u8>>, // AudioSpecificConfig
    profile: Option<u8>,
    sampling_frequency_index: Option<u8>,
    channel_configuration: Option<u8>,
}

impl AacFileReader {
    // Increased from 8192 (8KB) to 65536 (64KB) for Phase 3 optimization:
    // - AAC frames are smaller (~200-400 bytes), so 64KB covers ~160-320 frames
    // - Reduces syscalls significantly
    // - Still modest memory usage (+56KB)
    const BUFFER_SIZE: usize = 65536;
    const MIN_ADTS_HEADER_SIZE: usize = 7;

    /// Sampling rate index mapping (from MPEG-4 AAC spec)
    const AAC_SAMPLING_RATES: [(u8, u32); 13] = [
        (0, 96000),
        (1, 88200),
        (2, 64000),
        (3, 48000),
        (4, 44100),
        (5, 32000),
        (6, 24000),
        (7, 22050),
        (8, 16000),
        (9, 12000),
        (10, 11025),
        (11, 8000),
        (12, 7350),
    ];

    /// Create a new AAC file reader from file path
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the AAC file in ADTS format
    /// * `sample_rate` - Expected sample rate (validated against ADTS)
    ///
    /// # Returns
    ///
    /// Result with AacFileReader or AacFileError
    pub async fn new(file_path: &str, sample_rate: u32) -> Result<Self, AacFileError> {
        let mut file = File::open(file_path).await?;
        let file_size = file.seek(SeekFrom::End(0)).await?;
        file.seek(SeekFrom::Start(0)).await?;

        // AAC-LC: 1024 samples/frame
        // @ 48kHz: 1024 samples / 48000 Hz * 1000 = ~21.3ms
        // @ 44.1kHz: 1024 samples / 44100 Hz * 1000 = ~23.2ms
        let frame_duration_ms = if sample_rate > 0 {
            (1024 * 1000) / sample_rate
        } else {
            21 // Default for 48kHz
        };

        Ok(Self {
            file,
            buffer: vec![0u8; Self::BUFFER_SIZE],
            parse_data: Vec::with_capacity(Self::BUFFER_SIZE),
            parse_start: 0,
            parse_eof: false,
            current_pos: 0,
            file_size,
            sample_rate,
            frame_duration_ms,
            audio_config: None,
            profile: None,
            sampling_frequency_index: None,
            channel_configuration: None,
        })
    }

    /// Read the next AAC frame from the file
    ///
    /// # Returns
    ///
    /// `Option<AacFrame>` if successful, None at EOF, or AacFileError
    pub async fn read_next_frame(&mut self) -> Result<Option<AacFrame>, AacFileError> {
        loop {
            if !self.ensure_parse_bytes(Self::MIN_ADTS_HEADER_SIZE).await? {
                return Ok(None);
            }

            let sync_index = self.find_and_seek_sync().await?;
            let Some(sync_index) = sync_index else {
                return Ok(None);
            };

            if sync_index > self.parse_start {
                self.consume_parse_bytes(sync_index - self.parse_start);
                continue;
            }

            if !self.validate_sync_bytes()? {
                self.consume_parse_bytes(1);
                continue;
            }

            let Some(header_info) = self.parse_adts_header().await? else {
                return Ok(None);
            };

            if header_info.frame_length < header_info.adts_header_size {
                return Err(AacFileError::InvalidFrameLength);
            }

            if !self.ensure_parse_bytes(header_info.frame_length).await? {
                return Ok(None);
            }

            return self.extract_frame(header_info).await;
        }
    }

    async fn find_and_seek_sync(&mut self) -> Result<Option<usize>, AacFileError> {
        match Self::find_sync_word(&self.parse_data, self.parse_start) {
            Some(index) => Ok(Some(index)),
            None => {
                if self.fill_parse_buffer().await? {
                    return Ok(Some(self.parse_start));
                }
                let remaining = self.parse_data.len().saturating_sub(self.parse_start);
                self.consume_parse_bytes(remaining);
                Ok(None)
            }
        }
    }

    fn validate_sync_bytes(&self) -> Result<bool, AacFileError> {
        let header =
            &self.parse_data[self.parse_start..self.parse_start + Self::MIN_ADTS_HEADER_SIZE];
        Ok((header[0] == 0xFF) && ((header[1] & 0xF0) == 0xF0))
    }

    async fn parse_adts_header(&mut self) -> Result<Option<AdtsHeaderInfo>, AacFileError> {
        let header = &self.parse_data[self.parse_start..];
        if header.len() < Self::MIN_ADTS_HEADER_SIZE {
            return Ok(None);
        }

        let protection_absent = (header[1] & 0x01) != 0;
        let adts_header_size = if protection_absent { 7 } else { 9 };

        if !self.ensure_parse_bytes(adts_header_size).await? {
            return Ok(None);
        }

        let header = &self.parse_data[self.parse_start..self.parse_start + adts_header_size];
        Ok(Some(AdtsHeaderInfo {
            frame_length: Self::extract_frame_length(header),
            adts_header_size,
            profile: ((header[2] >> 6) & 0x03) + 1,
            sampling_freq_index: (header[2] >> 2) & 0x0F,
            channel_config: ((header[2] & 0x01) << 2) | ((header[3] >> 6) & 0x03),
        }))
    }

    fn extract_frame_length(header: &[u8]) -> usize {
        (((header[3] & 0x03) as usize) << 11)
            | ((header[4] as usize) << 3)
            | ((header[5] as usize) >> 5)
    }

    async fn extract_frame(
        &mut self,
        header_info: AdtsHeaderInfo,
    ) -> Result<Option<AacFrame>, AacFileError> {
        let AdtsHeaderInfo {
            frame_length,
            adts_header_size,
            profile,
            sampling_freq_index,
            channel_config,
        } = header_info;

        self.cache_adts_parameters(profile, sampling_freq_index, channel_config);

        let actual_sample_rate = self.resolve_sample_rate(sampling_freq_index);
        self.update_sample_rate_if_changed(actual_sample_rate);

        let frame_end = self.parse_start + frame_length;
        let payload = self.parse_data[self.parse_start + adts_header_size..frame_end].to_vec();
        self.consume_parse_bytes(frame_length);

        Ok(Some(AacFrame {
            data: payload,
            adts_header_size,
            total_frame_size: frame_length,
            profile,
            sample_rate: actual_sample_rate,
            channels: channel_config,
        }))
    }

    fn cache_adts_parameters(&mut self, profile: u8, sampling_freq_index: u8, channel_config: u8) {
        if self.profile.is_none() {
            self.profile = Some(profile);
            self.sampling_frequency_index = Some(sampling_freq_index);
            self.channel_configuration = Some(channel_config);
        }
    }

    fn resolve_sample_rate(&self, sampling_freq_index: u8) -> u32 {
        Self::AAC_SAMPLING_RATES
            .iter()
            .find(|(idx, _)| *idx == sampling_freq_index)
            .map(|(_, rate)| *rate)
            .unwrap_or(self.sample_rate)
    }

    fn update_sample_rate_if_changed(&mut self, actual_sample_rate: u32) {
        if self.sample_rate == actual_sample_rate {
            return;
        }

        tracing::warn!(
            configured_rate = self.sample_rate,
            adts_rate = actual_sample_rate,
            "sample_rate_mismatch_using_adts"
        );
        self.sample_rate = actual_sample_rate;
        self.frame_duration_ms = Self::calculate_frame_duration(actual_sample_rate);
    }

    fn calculate_frame_duration(sample_rate: u32) -> u32 {
        if sample_rate > 0 {
            (1024 * 1000) / sample_rate
        } else {
            21
        }
    }
}

struct AdtsHeaderInfo {
    frame_length: usize,
    adts_header_size: usize,
    profile: u8,
    sampling_freq_index: u8,
    channel_config: u8,
}

impl AacFileReader {
    /// Extract AudioSpecificConfig from ADTS headers (for SDP/RTP)
    ///
    /// Reads the first frame's ADTS header to extract ASC.
    /// Caches result for subsequent calls.
    ///
    /// # Returns
    ///
    /// 2-byte AudioSpecificConfig or error
    pub async fn extract_audio_config(&mut self) -> Result<Vec<u8>, AacFileError> {
        // Return cached config if available
        if let Some(ref config) = self.audio_config {
            return Ok(config.clone());
        }

        // Save current position
        let saved_pos = self.current_pos;

        // Reset to beginning and read first frame
        self.reset().await?;

        let _frame = self
            .read_next_frame()
            .await?
            .ok_or(AacFileError::NoFramesFound)?;

        // Build AudioSpecificConfig from cached values
        let profile = self.profile.ok_or(AacFileError::InvalidAdtsHeader)?;
        let freq_idx = self
            .sampling_frequency_index
            .ok_or(AacFileError::InvalidAdtsHeader)?;
        let channels = self
            .channel_configuration
            .ok_or(AacFileError::InvalidAdtsHeader)?;

        // Encode ASC (2 bytes)
        // Format: [audioObjectType(5) | samplingFrequencyIndex(4)] [samplingFreqIndex(1) | channelConfiguration(4) | frameLengthFlag(1) | dependsOnCoreCoder(1) | extensionFlag(1)]
        let config = vec![
            (profile << 3) | (freq_idx >> 1),
            ((freq_idx & 0x01) << 7) | (channels << 3),
        ];

        self.audio_config = Some(config.clone());

        // Restore position
        self.seek_to_position(saved_pos).await?;

        Ok(config)
    }

    /// Get the sample rate in Hz
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get the frame duration in milliseconds
    /// AAC has fixed 1024 samples per frame
    pub fn frame_duration_ms(&self) -> u32 {
        self.frame_duration_ms
    }

    /// Reset file position to beginning
    pub async fn reset(&mut self) -> Result<(), AacFileError> {
        self.seek_to_position(0).await?;
        Ok(())
    }

    /// Get current file position
    pub fn current_position(&self) -> u64 {
        self.current_pos
    }

    /// Get total file size
    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    async fn seek_to_position(&mut self, position: u64) -> Result<(), AacFileError> {
        self.file.seek(SeekFrom::Start(position)).await?;
        self.current_pos = position;
        self.parse_data.clear();
        self.parse_start = 0;
        self.parse_eof = false;
        Ok(())
    }

    async fn ensure_parse_bytes(&mut self, required: usize) -> Result<bool, AacFileError> {
        while self.parse_data.len().saturating_sub(self.parse_start) < required {
            if !self.fill_parse_buffer().await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn fill_parse_buffer(&mut self) -> Result<bool, AacFileError> {
        if self.parse_eof {
            return Ok(false);
        }

        let bytes_read = self.file.read(&mut self.buffer).await?;
        if bytes_read == 0 {
            self.parse_eof = true;
            return Ok(false);
        }

        self.parse_data
            .extend_from_slice(&self.buffer[..bytes_read]);
        Ok(true)
    }

    fn consume_parse_bytes(&mut self, count: usize) {
        let available = self.parse_data.len().saturating_sub(self.parse_start);
        let consumed = count.min(available);
        self.parse_start += consumed;
        self.current_pos = self.current_pos.saturating_add(consumed as u64);
        self.compact_parse_buffer();
    }

    fn compact_parse_buffer(&mut self) {
        if self.parse_start == 0 {
            return;
        }
        if self.parse_start >= self.parse_data.len() {
            self.parse_data.clear();
            self.parse_start = 0;
            return;
        }
        if self.parse_start >= Self::BUFFER_SIZE {
            self.parse_data.drain(..self.parse_start);
            self.parse_start = 0;
        }
    }

    fn find_sync_word(data: &[u8], from: usize) -> Option<usize> {
        if data.len() < 2 || from >= data.len() {
            return None;
        }

        let mut i = from;
        while i + 1 < data.len() {
            if data[i] == 0xFF && (data[i + 1] & 0xF0) == 0xF0 && (data[i + 1] & 0x06) == 0x00 {
                return Some(i);
            }
            i += 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_frame_duration_calculation() {
        let reader = AacFileReader::new("/dev/null", 48000).await.unwrap();
        assert_eq!(reader.frame_duration_ms(), 21); // 1024/48000 * 1000 = 21.3ms

        let reader = AacFileReader::new("/dev/null", 44100).await.unwrap();
        assert_eq!(reader.frame_duration_ms(), 23); // 1024/44100 * 1000 = 23.2ms
    }

    #[test]
    fn test_sample_rate_mapping() {
        // Verify sampling rate index mapping
        assert_eq!(
            AacFileReader::AAC_SAMPLING_RATES
                .iter()
                .find(|(idx, _)| *idx == 3)
                .map(|(_, rate)| *rate),
            Some(48000)
        );

        assert_eq!(
            AacFileReader::AAC_SAMPLING_RATES
                .iter()
                .find(|(idx, _)| *idx == 4)
                .map(|(_, rate)| *rate),
            Some(44100)
        );
    }

    #[tokio::test]
    async fn test_extract_audio_config_updates_sample_rate_from_adts() {
        let path = format!("/tmp/aac_sample_rate_detect_{}.aac", std::process::id());
        let bytes: &[u8] = &[
            0xFF, 0xF1, 0x50, 0x80, 0x01, 0x7F, 0xFC, // ADTS header (AAC-LC, 44.1kHz, stereo)
            0x00, 0x00, 0x00, 0x00, // payload
        ];
        std::fs::write(&path, bytes).expect("write test aac");

        let mut reader = AacFileReader::new(&path, 48_000).await.expect("reader new");
        let _ = reader.extract_audio_config().await.expect("extract config");
        assert_eq!(
            reader.sample_rate(),
            44_100,
            "reader should use sample rate from ADTS header"
        );

        let _ = std::fs::remove_file(&path);
    }

    // ========== find_sync_word tests ==========

    #[test]
    fn test_find_sync_word_at_start() {
        // ADTS sync word: 0xFF 0xF1 (0xFFF with layer=0, protection_absent=1)
        let data = [0xFF, 0xF1, 0x50, 0x80];
        let result = AacFileReader::find_sync_word(&data, 0);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_find_sync_word_with_offset() {
        let data = [0xAA, 0xBB, 0xFF, 0xF1, 0x50];
        let result = AacFileReader::find_sync_word(&data, 0);
        assert_eq!(result, Some(2));
    }

    #[test]
    fn test_find_sync_word_no_match() {
        let data = [0xAA, 0xBB, 0xCC, 0xDD];
        let result = AacFileReader::find_sync_word(&data, 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_sync_word_empty() {
        let result = AacFileReader::find_sync_word(&[], 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_sync_word_too_short() {
        let result = AacFileReader::find_sync_word(&[0xFF], 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_sync_word_from_past_end() {
        let data = [0xFF, 0xF1, 0x50];
        let result = AacFileReader::find_sync_word(&data, 10);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_sync_word_false_positive_rejected() {
        // 0xFF 0xFE has layer bits set (not 0), should not match
        // find_sync_word checks: data[i+1] & 0x06 == 0x00 (layer must be 0)
        let data = [0xFF, 0xFE, 0x50, 0x80];
        let result = AacFileReader::find_sync_word(&data, 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_sync_word_protection_absent_0() {
        // 0xFF 0xF0 — protection_absent=0, should still match (sync = 0xFFF0 with layer=0)
        let data = [0xFF, 0xF0, 0x50, 0x80];
        let result = AacFileReader::find_sync_word(&data, 0);
        assert_eq!(result, Some(0));
    }

    // ========== extract_frame_length tests ==========

    #[test]
    fn test_extract_frame_length_basic() {
        // Frame length is encoded across header[3..6]:
        // bits: header[3]&0x03 (2 bits) | header[4] (8 bits) | header[5]>>5 (3 bits) = 13 bits
        // For a frame length of 11 (0b00000001011):
        // header[3] & 0x03 = 0b00 (bits 12-11)
        // header[4] = 0b00000001 (bits 10-3)
        // header[5] >> 5 = 0b011 (bits 2-0)
        // length = (0 << 11) | (1 << 3) | 3 = 11
        let header = [0xFF, 0xF1, 0x50, 0x80, 0x01, 0x60, 0xFC];
        let length = AacFileReader::extract_frame_length(&header);
        assert_eq!(length, 11);
    }

    #[test]
    fn test_extract_frame_length_7_bytes() {
        // Standard ADTS header for a 7-byte frame (header only, no payload)
        // frame_length = 7: bits = 0b0_00000000_111
        // header[3] & 0x03 = 0 (0b00)
        // header[4] = 0 (0b00000000)
        // header[5] >> 5 = 7 (0b111)
        let header = [0xFF, 0xF1, 0x50, 0x80, 0x00, 0xE0, 0xFC];
        let length = AacFileReader::extract_frame_length(&header);
        assert_eq!(length, 7);
    }

    // ========== calculate_frame_duration tests ==========

    #[test]
    fn test_calculate_frame_duration_48khz() {
        assert_eq!(AacFileReader::calculate_frame_duration(48000), 21);
    }

    #[test]
    fn test_calculate_frame_duration_44100() {
        assert_eq!(AacFileReader::calculate_frame_duration(44100), 23);
    }

    #[test]
    fn test_calculate_frame_duration_16khz() {
        assert_eq!(AacFileReader::calculate_frame_duration(16000), 64);
    }

    #[test]
    fn test_calculate_frame_duration_8khz() {
        assert_eq!(AacFileReader::calculate_frame_duration(8000), 128);
    }

    #[test]
    fn test_calculate_frame_duration_zero_rate() {
        assert_eq!(AacFileReader::calculate_frame_duration(0), 21);
    }

    // ========== AAC_SAMPLING_RATES constant tests ==========

    #[test]
    fn test_aac_sampling_rates_complete() {
        let rates = &AacFileReader::AAC_SAMPLING_RATES;
        assert_eq!(rates.len(), 13);
        assert_eq!(rates[0], (0, 96000));
        assert_eq!(rates[4], (4, 44100));
        assert_eq!(rates[11], (11, 8000));
        assert_eq!(rates[12], (12, 7350));
    }

    #[test]
    fn test_aac_sampling_rates_indices_are_sequential() {
        for (i, (idx, _)) in AacFileReader::AAC_SAMPLING_RATES.iter().enumerate() {
            assert_eq!(*idx as usize, i);
        }
    }

    #[test]
    fn test_aac_sampling_rates_descending() {
        let rates: Vec<u32> = AacFileReader::AAC_SAMPLING_RATES
            .iter()
            .map(|(_, r)| *r)
            .collect();
        for i in 1..rates.len() {
            assert!(
                rates[i] < rates[i - 1],
                "Rates should be in descending order"
            );
        }
    }

    // ========== AacFileError display tests ==========

    #[test]
    fn test_aac_file_error_display() {
        let err = AacFileError::InvalidAdtsHeader;
        assert_eq!(format!("{err}"), "Invalid ADTS header");

        let err = AacFileError::InvalidFrameLength;
        assert_eq!(format!("{err}"), "Invalid ADTS frame length");

        let err = AacFileError::NoFramesFound;
        assert_eq!(format!("{err}"), "No frames found in file");

        let err = AacFileError::UnsupportedProfile(5);
        assert_eq!(format!("{err}"), "Unsupported AAC profile: 5");

        let err = AacFileError::UnsupportedSamplingFrequency(15);
        assert_eq!(format!("{err}"), "Unsupported sampling frequency index: 15");

        let err = AacFileError::InvalidSyncWord;
        assert_eq!(format!("{err}"), "Invalid sync word");
    }

    #[test]
    fn test_aac_file_error_io_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let aac_err: AacFileError = io_err.into();
        assert!(format!("{aac_err}").contains("no such file"));
    }

    // ========== AacFrame struct tests ==========

    #[test]
    fn test_aac_frame_clone() {
        let frame = AacFrame {
            data: vec![0xAA, 0xBB],
            adts_header_size: 7,
            total_frame_size: 9,
            profile: 2,
            sample_rate: 44100,
            channels: 2,
        };
        let cloned = frame.clone();
        assert_eq!(cloned.data, vec![0xAA, 0xBB]);
        assert_eq!(cloned.adts_header_size, 7);
        assert_eq!(cloned.total_frame_size, 9);
        assert_eq!(cloned.profile, 2);
        assert_eq!(cloned.sample_rate, 44100);
        assert_eq!(cloned.channels, 2);
    }

    #[test]
    fn test_aac_frame_debug() {
        let frame = AacFrame {
            data: vec![0xAA],
            adts_header_size: 7,
            total_frame_size: 8,
            profile: 2,
            sample_rate: 48000,
            channels: 1,
        };
        let debug = format!("{frame:?}");
        assert!(debug.contains("AacFrame"));
        assert!(debug.contains("48000"));
    }

    // ========== Async reader property tests ==========

    #[tokio::test]
    async fn test_reader_initial_properties() {
        let reader = AacFileReader::new("/dev/null", 48000).await.unwrap();
        assert_eq!(reader.sample_rate(), 48000);
        assert_eq!(reader.frame_duration_ms(), 21);
        assert_eq!(reader.current_position(), 0);
        assert_eq!(reader.file_size(), 0);
    }

    #[tokio::test]
    async fn test_reader_zero_sample_rate_defaults() {
        let reader = AacFileReader::new("/dev/null", 0).await.unwrap();
        assert_eq!(reader.frame_duration_ms(), 21); // Default for 48kHz
    }

    #[tokio::test]
    async fn test_read_frame_from_empty_file() {
        let mut reader = AacFileReader::new("/dev/null", 48000).await.unwrap();
        let result = reader.read_next_frame().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_read_frame_from_valid_adts() {
        let path = format!("/tmp/aac_valid_frame_{}.aac", std::process::id());
        // Construct a valid ADTS frame:
        // Sync word: 0xFFF, MPEG-4=1, Layer=0, Protection_absent=1
        // Profile: AAC-LC (1 -> object type 2, stored as 01 in 2 bits)
        // Sampling freq idx: 3 (48kHz)
        // Channel config: 2 (stereo)
        // Frame length: 11 (7 header + 4 payload)
        // header[3..5] encode frame length in 13 bits
        let header = [
            0xFF, 0xF1, // sync + MPEG-4, layer=0, protection_absent=1
            0x50, // profile=01(LC), freq_idx=0011(48k), private=0, channel high bit=0
            0x80, // channel low 2 bits=10(stereo), originality=0, home=0, ...
            0x01, // ...frame_length bits continued
            0x60, // ...frame_length bits = 11 total
            0xFC, // buffer fullness + frame count
        ];
        let payload = [0xAA, 0xBB, 0xCC, 0xDD];
        let mut data = Vec::new();
        data.extend_from_slice(&header);
        data.extend_from_slice(&payload);
        std::fs::write(&path, &data).expect("write test aac");

        let mut reader = AacFileReader::new(&path, 48000).await.expect("open");
        let frame = reader.read_next_frame().await.unwrap();
        assert!(frame.is_some());
        let frame = frame.unwrap();
        assert_eq!(frame.channels, 2);
        assert_eq!(frame.adts_header_size, 7);

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn test_reset_allows_rereading() {
        let path = format!("/tmp/aac_reset_{}.aac", std::process::id());
        let bytes: &[u8] = &[
            0xFF, 0xF1, 0x50, 0x80, 0x01, 0x7F, 0xFC, // ADTS header
            0x00, 0x00, 0x00, 0x00, // payload
        ];
        std::fs::write(&path, bytes).expect("write");

        let mut reader = AacFileReader::new(&path, 48000).await.unwrap();
        let frame1 = reader.read_next_frame().await.unwrap();
        assert!(frame1.is_some());

        reader.reset().await.unwrap();
        assert_eq!(reader.current_position(), 0);

        let frame2 = reader.read_next_frame().await.unwrap();
        assert!(frame2.is_some());

        let _ = std::fs::remove_file(&path);
    }
}
