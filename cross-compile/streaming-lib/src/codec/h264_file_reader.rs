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
    nal_format: NalFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NalFormat {
    AnnexB,
    Avcc,
}

impl H264FileReader {
    const BUFFER_SIZE: usize = 65536;
    const START_CODE_3: &'static [u8] = &[0x00, 0x00, 0x01];
    const START_CODE_4: &'static [u8] = &[0x00, 0x00, 0x00, 0x01];
    const MAX_PARAM_SET_LEN: usize = 4096;

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

        let nal_format = Self::detect_nal_format(&mut file, file_size)?;

        Ok(Self {
            file,
            buffer: vec![0u8; Self::BUFFER_SIZE],
            current_pos: 0,
            file_size,
            frame_rate,
            frame_duration_ms,
            nal_format,
        })
    }

    fn detect_nal_format(file: &mut File, file_size: u64) -> Result<NalFormat, H264FileError> {
        let detect_len = std::cmp::min(
            Self::BUFFER_SIZE + Self::START_CODE_4.len(),
            file_size as usize,
        );
        let mut header = vec![0u8; detect_len];
        let bytes_read = file.read(&mut header)?;
        header.truncate(bytes_read);
        file.seek(SeekFrom::Start(0))?;

        let has_start_code_4 = header
            .windows(Self::START_CODE_4.len())
            .any(|w| w == Self::START_CODE_4);
        let has_start_code_3 = header
            .windows(Self::START_CODE_3.len())
            .any(|w| w == Self::START_CODE_3);

        if has_start_code_4 || has_start_code_3 {
            Ok(NalFormat::AnnexB)
        } else {
            Ok(NalFormat::Avcc)
        }
    }

    /// Read the next NAL unit from the file
    ///
    /// # Returns
    ///
    /// Option<NalUnit> if successful, None at EOF, or H264FileError
    pub fn read_next_nal(&mut self) -> Result<Option<NalUnit>, H264FileError> {
        match self.nal_format {
            NalFormat::AnnexB => self.read_next_nal_annexb(),
            NalFormat::Avcc => self.read_next_nal_avcc(),
        }
    }

    fn read_next_nal_annexb(&mut self) -> Result<Option<NalUnit>, H264FileError> {
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

    fn read_next_nal_avcc(&mut self) -> Result<Option<NalUnit>, H264FileError> {
        if self.current_pos >= self.file_size {
            return Ok(None);
        }

        let mut len_buf = [0u8; 4];
        let bytes_read = self.file.read(&mut len_buf)?;
        if bytes_read < 4 {
            return Ok(None);
        }
        self.current_pos += 4;

        let nal_len = u32::from_be_bytes(len_buf) as usize;
        if nal_len == 0 || self.current_pos + nal_len as u64 > self.file_size {
            return Err(H264FileError::InvalidNalUnit);
        }

        let mut nal_data = vec![0u8; nal_len];
        let data_read = self.file.read(&mut nal_data)?;
        if data_read != nal_len {
            nal_data.truncate(data_read);
        }
        self.current_pos += data_read as u64;

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
            start_code_length: 0,
        }))
    }

    /// Find the position and length of the next start code in buffer
    ///
    /// Returns (offset_in_buffer, start_code_length)
    /// Seeks back so file pointer is positioned just before the start code.
    /// If no start code found at EOF, returns (0, 0) and rewinds all read bytes.
    fn find_next_start_code(&mut self) -> Result<(usize, usize), H264FileError> {
        let start_pos = self.current_pos;
        let mut total_read: usize = 0;
        let mut tail_len: usize = 0;

        loop {
            let bytes_read = self.file.read(&mut self.buffer[tail_len..])?;
            if bytes_read == 0 {
                if total_read > 0 {
                    self.file.seek(SeekFrom::Current(-(total_read as i64)))?;
                }
                return Ok((0, 0));
            }

            let buf_len = tail_len + bytes_read;
            let base_offset = total_read.saturating_sub(tail_len);

            // Search for 4-byte start code first
            for i in 0..buf_len.saturating_sub(3) {
                if self.buffer[i..i + 4] == *Self::START_CODE_4 {
                    let offset = base_offset + i;
                    let seek_back = (total_read + bytes_read).saturating_sub(offset) as i64;
                    self.file.seek(SeekFrom::Current(-seek_back))?;
                    self.current_pos = start_pos + offset as u64;
                    return Ok((offset, 4));
                }
            }

            // Search for 3-byte start code
            for i in 0..buf_len.saturating_sub(2) {
                if self.buffer[i..i + 3] == *Self::START_CODE_3 {
                    let offset = base_offset + i;
                    let seek_back = (total_read + bytes_read).saturating_sub(offset) as i64;
                    self.file.seek(SeekFrom::Current(-seek_back))?;
                    self.current_pos = start_pos + offset as u64;
                    return Ok((offset, 3));
                }
            }

            total_read += bytes_read;

            // No start code found in this chunk
            if bytes_read < Self::BUFFER_SIZE.saturating_sub(tail_len) {
                // End of file reached without finding start code - rewind all read bytes
                self.file.seek(SeekFrom::Current(-(total_read as i64)))?;
                return Ok((0, 0));
            }

            // Preserve tail for start codes spanning the buffer boundary
            tail_len = 3.min(buf_len);
            self.buffer.copy_within(buf_len - tail_len..buf_len, 0);
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

        let scan_len = std::cmp::min(self.file_size as usize, 1024 * 1024);
        let mut scan_buf = vec![0u8; scan_len];
        let bytes_read = self.file.read(&mut scan_buf)?;
        scan_buf.truncate(bytes_read);

        let scan_result = Self::scan_sps_pps_from_buffer(&scan_buf);

        self.file.seek(SeekFrom::Start(0))?;
        self.current_pos = 0;

        let (sps, pps) = if let Some(found) = scan_result {
            found
        } else {
            let mut sps = Vec::new();
            let mut pps = Vec::new();

            while let Some(nal) = self.read_next_nal()? {
                match nal.unit_type {
                    NalUnitType::SequenceParameterSet => {
                        if nal.data.len() <= Self::MAX_PARAM_SET_LEN {
                            sps = nal.data.clone();
                        }
                    }
                    NalUnitType::PictureParameterSet => {
                        if nal.data.len() <= Self::MAX_PARAM_SET_LEN {
                            pps = nal.data.clone();
                        }
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

            (sps, pps)
        };

        self.file.seek(SeekFrom::Start(0))?;
        self.current_pos = 0;

        if sps.is_empty() || pps.is_empty() {
            return Err(H264FileError::NoNalUnits);
        }

        Ok((sps, pps))
    }

    fn scan_sps_pps_from_buffer(buffer: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
        if let Some(found) = Self::scan_annexb_for_sps_pps(buffer) {
            return Some(found);
        }
        Self::scan_avcc_for_sps_pps(buffer)
    }

    fn scan_annexb_for_sps_pps(buffer: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
        let mut start_codes: Vec<(usize, usize)> = Vec::new();
        let mut i = 0;
        while i + 3 < buffer.len() {
            if i + 4 <= buffer.len() && buffer[i..i + 4] == *Self::START_CODE_4 {
                start_codes.push((i, 4));
                i += 4;
                continue;
            }
            if buffer[i..i + 3] == *Self::START_CODE_3 {
                start_codes.push((i, 3));
                i += 3;
                continue;
            }
            i += 1;
        }

        let mut sps: Option<Vec<u8>> = None;
        let mut pps: Option<Vec<u8>> = None;

        for idx in 0..start_codes.len() {
            let (start_pos, sc_len) = start_codes[idx];
            let data_start = start_pos + sc_len;
            let data_end = if idx + 1 < start_codes.len() {
                start_codes[idx + 1].0
            } else {
                buffer.len()
            };

            if data_start >= data_end {
                continue;
            }

            let nal = &buffer[data_start..data_end];
            if nal.is_empty() {
                continue;
            }

            if nal.len() > Self::MAX_PARAM_SET_LEN {
                continue;
            }

            match nal[0] & 0x1f {
                7 => {
                    sps = Some(nal.to_vec());
                }
                8 => {
                    pps = Some(nal.to_vec());
                }
                _ => {}
            }

            if sps.is_some() && pps.is_some() {
                break;
            }
        }

        match (sps, pps) {
            (Some(sps), Some(pps)) => Some((sps, pps)),
            _ => None,
        }
    }

    fn scan_avcc_for_sps_pps(buffer: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
        let mut sps: Option<Vec<u8>> = None;
        let mut pps: Option<Vec<u8>> = None;
        let mut offset = 0usize;

        while offset + 4 <= buffer.len() {
            let len = u32::from_be_bytes([
                buffer[offset],
                buffer[offset + 1],
                buffer[offset + 2],
                buffer[offset + 3],
            ]) as usize;
            offset += 4;

            if len == 0 || offset + len > buffer.len() {
                break;
            }

            let nal = &buffer[offset..offset + len];
            offset += len;

            if nal.is_empty() || nal.len() > Self::MAX_PARAM_SET_LEN {
                continue;
            }

            match nal[0] & 0x1f {
                7 => {
                    sps = Some(nal.to_vec());
                }
                8 => {
                    pps = Some(nal.to_vec());
                }
                _ => {}
            }

            if sps.is_some() && pps.is_some() {
                break;
            }
        }

        match (sps, pps) {
            (Some(sps), Some(pps)) => Some((sps, pps)),
            _ => None,
        }
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
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn test_read_next_nal_start_code_across_buffer_boundary() {
        let mut data = vec![0u8; H264FileReader::BUFFER_SIZE - 2];
        data.extend_from_slice(&[0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1e]);
        data.extend_from_slice(&[0x00, 0x00, 0x01, 0x68, 0xce, 0x06, 0xe2]);

        let temp_dir = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file_path = temp_dir.join(format!("h264_boundary_test_{nanos}.h264"));
        let mut temp_file = std::fs::File::create(&file_path).unwrap();
        temp_file.write_all(&data).unwrap();
        drop(temp_file);

        let mut reader = H264FileReader::new(file_path.to_str().unwrap(), 25).unwrap();
        let sps = reader.read_next_nal().unwrap().unwrap();
        assert_eq!(sps.unit_type, NalUnitType::SequenceParameterSet);
        assert_eq!(sps.data, vec![0x67, 0x42, 0x00, 0x1e]);

        let pps = reader.read_next_nal().unwrap().unwrap();
        assert_eq!(pps.unit_type, NalUnitType::PictureParameterSet);
        assert_eq!(pps.data, vec![0x68, 0xce, 0x06, 0xe2]);

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_read_next_nal_avcc_format() {
        let sps = [0x67, 0x42, 0x00, 0x1e];
        let pps = [0x68, 0xce, 0x06, 0xe2];

        let mut data = Vec::new();
        data.extend_from_slice(&(sps.len() as u32).to_be_bytes());
        data.extend_from_slice(&sps);
        data.extend_from_slice(&(pps.len() as u32).to_be_bytes());
        data.extend_from_slice(&pps);

        let temp_dir = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file_path = temp_dir.join(format!("h264_avcc_test_{nanos}.h264"));
        let mut temp_file = std::fs::File::create(&file_path).unwrap();
        temp_file.write_all(&data).unwrap();
        drop(temp_file);

        let mut reader = H264FileReader::new(file_path.to_str().unwrap(), 25).unwrap();
        let sps_nal = reader.read_next_nal().unwrap().unwrap();
        assert_eq!(sps_nal.unit_type, NalUnitType::SequenceParameterSet);
        assert_eq!(sps_nal.data, sps);

        let pps_nal = reader.read_next_nal().unwrap().unwrap();
        assert_eq!(pps_nal.unit_type, NalUnitType::PictureParameterSet);
        assert_eq!(pps_nal.data, pps);

        let _ = std::fs::remove_file(file_path);
    }
}
