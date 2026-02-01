use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use thiserror::Error;

/// Errors that can occur while reading H264 files
#[derive(Error, Debug)]
pub enum H264FileError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid H264 file format")]
    InvalidFormat,

    #[error("No NAL units found")]
    NoNalUnits,

    #[error("Invalid NAL unit")]
    InvalidNalUnit,
}

/// NAL unit types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NalUnitType {
    Unspecified,
    NonIdrSlice,
    PartitionASlice,
    PartitionBSlice,
    PartitionCSlice,
    IdrSlice,
    SupplementalEnhancementInformation,
    SequenceParameterSet,
    PictureParameterSet,
    AccessUnitDelimiter,
    EndOfSequence,
    EndOfStream,
    FillerData,
    SequenceParameterSetExtension,
    PrefixNalUnit,
    SubsetSequenceParameterSet,
    DepthParameterSet,
    Reserved,
    UnspecifiedHighValue,
}

impl From<u8> for NalUnitType {
    fn from(value: u8) -> Self {
        match value {
            1 => NalUnitType::NonIdrSlice,
            2 => NalUnitType::PartitionASlice,
            3 => NalUnitType::PartitionBSlice,
            4 => NalUnitType::PartitionCSlice,
            5 => NalUnitType::IdrSlice,
            6 => NalUnitType::SupplementalEnhancementInformation,
            7 => NalUnitType::SequenceParameterSet,
            8 => NalUnitType::PictureParameterSet,
            9 => NalUnitType::AccessUnitDelimiter,
            10 => NalUnitType::EndOfSequence,
            11 => NalUnitType::EndOfStream,
            12 => NalUnitType::FillerData,
            13 => NalUnitType::SequenceParameterSetExtension,
            14 => NalUnitType::PrefixNalUnit,
            15 => NalUnitType::SubsetSequenceParameterSet,
            16 => NalUnitType::DepthParameterSet,
            17..=18 => NalUnitType::Reserved,
            19..=23 => NalUnitType::Reserved,
            _ => NalUnitType::UnspecifiedHighValue,
        }
    }
}

/// Represents a parsed NAL unit from H264 stream
#[derive(Debug, Clone)]
pub struct NalUnit {
    pub unit_type: NalUnitType,
    pub data: Vec<u8>,
    pub start_code_length: usize,
}

/// H264 file reader for Annex-B format files
pub struct H264FileReader {
    file: File,
    buffer: Vec<u8>,
    current_pos: u64,
    file_size: u64,
    frame_rate: u32,
    frame_duration_ms: u32,
}

impl H264FileReader {
    const BUFFER_SIZE: usize = 65536;
    const START_CODE_3: &'static [u8] = &[0x00, 0x00, 0x01];
    const START_CODE_4: &'static [u8] = &[0x00, 0x00, 0x00, 0x01];

    /// Create a new H264 file reader from file path
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the H264 file in Annex-B format
    /// * `frame_rate` - Expected frame rate in fps (default 25fps = 40ms intervals)
    ///
    /// # Returns
    ///
    /// Result with H264FileReader or H264FileError
    pub fn new(file_path: &str, frame_rate: u32) -> Result<Self, H264FileError> {
        let mut file = File::open(file_path)?;
        let file_size = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(0))?;

        let frame_duration_ms = if frame_rate > 0 {
            1000 / frame_rate
        } else {
            40 // Default to 25fps
        };

        Ok(Self {
            file,
            buffer: vec![0u8; Self::BUFFER_SIZE],
            current_pos: 0,
            file_size,
            frame_rate,
            frame_duration_ms,
        })
    }

    /// Read the next NAL unit from the file
    ///
    /// # Returns
    ///
    /// Option<NalUnit> if successful, None at EOF, or H264FileError
    pub fn read_next_nal(&mut self) -> Result<Option<NalUnit>, H264FileError> {
        if self.current_pos >= self.file_size {
            return Ok(None);
        }

        // Find the next start code
        // Returns (offset_in_buffer, start_code_length)
        // File pointer is positioned just before the start code
        let (_offset, start_code_len) = self.find_next_start_code()?;

        // If start_code_len == 0, we've reached EOF without finding a start code
        if start_code_len == 0 {
            return Ok(None);
        }

        // Skip past the start code with relative seek
        self.file.seek(SeekFrom::Current(start_code_len as i64))?;
        self.current_pos += start_code_len as u64;

        // Find the next start code to know where this NAL unit ends
        let (next_offset, next_start_len) = self.find_next_start_code()?;

        // Determine how many bytes to read for this NAL unit
        let nal_data_len = if next_start_len == 0 {
            // EOF reached - read all remaining bytes as the final NAL unit
            (self.file_size - self.current_pos) as usize
        } else {
            // Next start code found - read up to it
            next_offset
        };

        // Handle empty NAL data at EOF
        if nal_data_len == 0 {
            return Ok(None);
        }

        // CRITICAL FIX: After find_next_start_code(), the file pointer is positioned
        // AT the next start code. We need to seek backward to read the NAL data
        // that comes BEFORE the next start code (from current_pos to next_start_code).
        if next_start_len != 0 {
            self.file.seek(SeekFrom::Current(-(nal_data_len as i64)))?;
            self.current_pos -= nal_data_len as u64;
        }

        // Read NAL unit data from the correct position
        let mut nal_data = vec![0u8; nal_data_len];
        let bytes_read = self.file.read(&mut nal_data)?;
        if bytes_read != nal_data_len {
            nal_data.truncate(bytes_read);
        }
        self.current_pos += bytes_read as u64;

        // Extract NAL unit type from first byte
        if nal_data.is_empty() {
            return Err(H264FileError::InvalidNalUnit);
        }

        let forbidden_bit = (nal_data[0] >> 7) & 1;
        if forbidden_bit != 0 {
            return Err(H264FileError::InvalidNalUnit);
        }

        let _nal_ref_idc = (nal_data[0] >> 5) & 3;
        let nal_unit_type = nal_data[0] & 0x1f;

        Ok(Some(NalUnit {
            unit_type: NalUnitType::from(nal_unit_type),
            data: nal_data,
            start_code_length: start_code_len,
        }))
    }

    /// Find the position and length of the next start code in buffer
    ///
    /// Returns (offset_in_buffer, start_code_length)
    /// Seeks back so file pointer is positioned just before the start code.
    /// If no start code found at EOF, returns (0, 0) and rewinds all read bytes.
    fn find_next_start_code(&mut self) -> Result<(usize, usize), H264FileError> {
        loop {
            let bytes_read = self.file.read(&mut self.buffer)?;
            if bytes_read == 0 {
                return Ok((0, 0)); // EOF
            }

            // Search for 4-byte start code first
            for i in 0..bytes_read.saturating_sub(3) {
                if self.buffer[i..i + 4] == *Self::START_CODE_4 {
                    // Seek back to position just before the start code
                    let rewind = (bytes_read - i) as i64;
                    self.file.seek(SeekFrom::Current(-rewind))?;
                    self.current_pos += i as u64;
                    return Ok((i, 4));
                }
            }

            // Search for 3-byte start code
            for i in 0..bytes_read.saturating_sub(2) {
                if self.buffer[i..i + 3] == *Self::START_CODE_3 {
                    // Seek back to position just before the start code
                    let rewind = (bytes_read - i) as i64;
                    self.file.seek(SeekFrom::Current(-rewind))?;
                    self.current_pos += i as u64;
                    return Ok((i, 3));
                }
            }

            // No start code found in this chunk
            if bytes_read < Self::BUFFER_SIZE {
                // End of file reached without finding start code - rewind all read bytes
                self.file.seek(SeekFrom::Current(-(bytes_read as i64)))?;
                return Ok((0, 0));
            }

            // More data to read, accumulate position
            self.current_pos += bytes_read as u64;
        }
    }

    /// Extract SPS (Sequence Parameter Set) from next NAL units
    ///
    /// # Returns
    ///
    /// Tuple of (SPS bytes, PPS bytes) or error
    pub fn extract_sps_pps(&mut self) -> Result<(Vec<u8>, Vec<u8>), H264FileError> {
        self.file.seek(SeekFrom::Start(0))?;
        self.current_pos = 0;

        let mut sps = Vec::new();
        let mut pps = Vec::new();

        while let Some(nal) = self.read_next_nal()? {
            match nal.unit_type {
                NalUnitType::SequenceParameterSet => {
                    sps = nal.data.clone();
                }
                NalUnitType::PictureParameterSet => {
                    pps = nal.data.clone();
                }
                NalUnitType::IdrSlice => {
                    break; // Stop after first IDR frame
                }
                _ => {}
            }

            if !sps.is_empty() && !pps.is_empty() {
                break;
            }
        }

        self.file.seek(SeekFrom::Start(0))?;
        self.current_pos = 0;

        if sps.is_empty() || pps.is_empty() {
            return Err(H264FileError::NoNalUnits);
        }

        Ok((sps, pps))
    }

    /// Get the frame rate in fps
    pub fn frame_rate(&self) -> u32 {
        self.frame_rate
    }

    /// Get the frame duration in milliseconds
    pub fn frame_duration_ms(&self) -> u32 {
        self.frame_duration_ms
    }

    /// Reset file position to beginning
    pub fn reset(&mut self) -> Result<(), H264FileError> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nal_unit_type_conversion() {
        assert_eq!(NalUnitType::from(1), NalUnitType::NonIdrSlice);
        assert_eq!(NalUnitType::from(5), NalUnitType::IdrSlice);
        assert_eq!(NalUnitType::from(7), NalUnitType::SequenceParameterSet);
        assert_eq!(NalUnitType::from(8), NalUnitType::PictureParameterSet);
    }

    #[test]
    fn test_frame_duration_calculation() {
        let reader = H264FileReader::new("/dev/null", 25).unwrap();
        assert_eq!(reader.frame_duration_ms(), 40);

        let reader = H264FileReader::new("/dev/null", 30).unwrap();
        assert_eq!(reader.frame_duration_ms(), 33);

        let reader = H264FileReader::new("/dev/null", 0).unwrap();
        assert_eq!(reader.frame_duration_ms(), 40); // Default fallback
    }
}
