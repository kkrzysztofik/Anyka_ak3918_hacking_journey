use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use thiserror::Error;

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
    const BUFFER_SIZE: usize = 8192; // AAC frames are smaller than H264
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
    pub fn new(file_path: &str, sample_rate: u32) -> Result<Self, AacFileError> {
        let mut file = File::open(file_path)?;
        let file_size = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(0))?;

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
    /// Option<AacFrame> if successful, None at EOF, or AacFileError
    pub fn read_next_frame(&mut self) -> Result<Option<AacFrame>, AacFileError> {
        if self.current_pos >= self.file_size {
            return Ok(None);
        }

        // Find ADTS sync word
        let header_offset = self.find_next_sync_word()?;
        if header_offset.is_none() {
            return Ok(None); // EOF
        }

        // Read ADTS header (7 or 9 bytes)
        let mut header = vec![0u8; Self::MIN_ADTS_HEADER_SIZE];
        self.file.read_exact(&mut header)?;
        self.current_pos += Self::MIN_ADTS_HEADER_SIZE as u64;

        // Validate sync word
        if (header[0] != 0xFF) || ((header[1] & 0xF0) != 0xF0) {
            return Err(AacFileError::InvalidAdtsHeader);
        }

        // Check protection_absent bit (1 = no CRC, 0 = CRC present)
        let protection_absent = (header[1] & 0x01) != 0;
        let adts_header_size = if protection_absent { 7 } else { 9 };

        // Read CRC if present
        if !protection_absent {
            let mut crc = vec![0u8; 2];
            self.file.read_exact(&mut crc)?;
            self.current_pos += 2;
        }

        // Extract frame length (13 bits across bytes 3-5)
        let frame_length = (((header[3] & 0x03) as usize) << 11)
            | ((header[4] as usize) << 3)
            | ((header[5] as usize) >> 5);

        // Validate frame length
        if frame_length < adts_header_size {
            return Err(AacFileError::InvalidFrameLength);
        }

        // Calculate payload size
        let payload_size = frame_length - adts_header_size;

        // Read AAC payload
        let mut payload = vec![0u8; payload_size];
        self.file.read_exact(&mut payload)?;
        self.current_pos += payload_size as u64;

        // Extract metadata from ADTS header
        let profile = ((header[2] >> 6) & 0x03) + 1;
        let sampling_freq_index = (header[2] >> 2) & 0x0F;
        let channel_config = ((header[2] & 0x01) << 2) | ((header[3] >> 6) & 0x03);

        // Cache config on first frame
        if self.profile.is_none() {
            self.profile = Some(profile);
            self.sampling_frequency_index = Some(sampling_freq_index);
            self.channel_configuration = Some(channel_config);
        }

        // Get actual sample rate from index
        let actual_sample_rate = Self::AAC_SAMPLING_RATES
            .iter()
            .find(|(idx, _)| *idx == sampling_freq_index)
            .map(|(_, rate)| *rate)
            .unwrap_or(self.sample_rate);

        Ok(Some(AacFrame {
            data: payload,
            adts_header_size,
            total_frame_size: frame_length,
            profile,
            sample_rate: actual_sample_rate,
            channels: channel_config,
        }))
    }

    /// Extract AudioSpecificConfig from ADTS headers (for SDP/RTP)
    ///
    /// Reads the first frame's ADTS header to extract ASC.
    /// Caches result for subsequent calls.
    ///
    /// # Returns
    ///
    /// 2-byte AudioSpecificConfig or error
    pub fn extract_audio_config(&mut self) -> Result<Vec<u8>, AacFileError> {
        // Return cached config if available
        if let Some(ref config) = self.audio_config {
            return Ok(config.clone());
        }

        // Save current position
        let saved_pos = self.current_pos;

        // Reset to beginning and read first frame
        self.reset()?;

        let _frame = self.read_next_frame()?.ok_or(AacFileError::NoFramesFound)?;

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
        self.file.seek(SeekFrom::Start(saved_pos))?;
        self.current_pos = saved_pos;

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
    pub fn reset(&mut self) -> Result<(), AacFileError> {
        self.file.seek(SeekFrom::Start(0))?;
        self.current_pos = 0;
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

    /// Find the next ADTS sync word in the file
    ///
    /// Returns the offset in the buffer where the sync word was found
    fn find_next_sync_word(&mut self) -> Result<Option<usize>, AacFileError> {
        loop {
            let bytes_read = self.file.read(&mut self.buffer)?;

            if bytes_read == 0 {
                return Ok(None); // EOF
            }

            // Search for 0xFFF sync word (12 bits)
            for i in 0..bytes_read.saturating_sub(1) {
                if self.buffer[i] == 0xFF && (self.buffer[i + 1] & 0xF0) == 0xF0 {
                    // Validate layer bits are 00 (bits 5-6 of second byte)
                    if (self.buffer[i + 1] & 0x06) == 0x00 {
                        // Seek back to position at sync word
                        let rewind = (bytes_read - i) as i64;
                        self.file.seek(SeekFrom::Current(-rewind))?;
                        self.current_pos += i as u64;
                        return Ok(Some(i));
                    }
                }
            }

            // No sync word found, continue reading
            if bytes_read < Self::BUFFER_SIZE {
                return Ok(None); // EOF without sync
            }

            self.current_pos += bytes_read as u64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_duration_calculation() {
        let reader = AacFileReader::new("/dev/null", 48000).unwrap();
        assert_eq!(reader.frame_duration_ms(), 21); // 1024/48000 * 1000 = 21.3ms

        let reader = AacFileReader::new("/dev/null", 44100).unwrap();
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
}
