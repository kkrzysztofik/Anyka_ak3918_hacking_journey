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

            let sync_index = match Self::find_sync_word(&self.parse_data, self.parse_start) {
                Some(index) => index,
                None => {
                    if self.fill_parse_buffer().await? {
                        continue;
                    }
                    let remaining = self.parse_data.len().saturating_sub(self.parse_start);
                    self.consume_parse_bytes(remaining);
                    return Ok(None);
                }
            };

            if sync_index > self.parse_start {
                self.consume_parse_bytes(sync_index - self.parse_start);
                continue;
            }

            let header =
                &self.parse_data[self.parse_start..self.parse_start + Self::MIN_ADTS_HEADER_SIZE];
            if (header[0] != 0xFF) || ((header[1] & 0xF0) != 0xF0) {
                self.consume_parse_bytes(1);
                continue;
            }

            let protection_absent = (header[1] & 0x01) != 0;
            let adts_header_size = if protection_absent { 7 } else { 9 };
            if !self.ensure_parse_bytes(adts_header_size).await? {
                return Ok(None);
            }

            let header = &self.parse_data[self.parse_start..self.parse_start + adts_header_size];
            let frame_length = (((header[3] & 0x03) as usize) << 11)
                | ((header[4] as usize) << 3)
                | ((header[5] as usize) >> 5);
            let profile = ((header[2] >> 6) & 0x03) + 1;
            let sampling_freq_index = (header[2] >> 2) & 0x0F;
            let channel_config = ((header[2] & 0x01) << 2) | ((header[3] >> 6) & 0x03);

            if frame_length < adts_header_size {
                return Err(AacFileError::InvalidFrameLength);
            }

            if !self.ensure_parse_bytes(frame_length).await? {
                return Ok(None);
            }

            let frame_end = self.parse_start + frame_length;
            let payload = self.parse_data[self.parse_start + adts_header_size..frame_end].to_vec();

            if self.profile.is_none() {
                self.profile = Some(profile);
                self.sampling_frequency_index = Some(sampling_freq_index);
                self.channel_configuration = Some(channel_config);
            }

            let actual_sample_rate = Self::AAC_SAMPLING_RATES
                .iter()
                .find(|(idx, _)| *idx == sampling_freq_index)
                .map(|(_, rate)| *rate)
                .unwrap_or(self.sample_rate);

            if self.sample_rate != actual_sample_rate {
                log::warn!(
                    "aac_file_reader: configured sample_rate={} differs from ADTS sample_rate={}, using ADTS value",
                    self.sample_rate,
                    actual_sample_rate
                );
                self.sample_rate = actual_sample_rate;
                self.frame_duration_ms = if actual_sample_rate > 0 {
                    (1024 * 1000) / actual_sample_rate
                } else {
                    self.frame_duration_ms
                };
            }

            self.consume_parse_bytes(frame_length);

            return Ok(Some(AacFrame {
                data: payload,
                adts_header_size,
                total_frame_size: frame_length,
                profile,
                sample_rate: actual_sample_rate,
                channels: channel_config,
            }));
        }
    }

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
}
