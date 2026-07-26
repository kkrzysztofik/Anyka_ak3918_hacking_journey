use thiserror::Error;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};

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
    annexb_data: Vec<u8>,
    annexb_start: usize,
    annexb_eof: bool,
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
    // Increased from 65536 (64KB) to 524288 (512KB) for Phase 3 optimization:
    // - Reduces syscalls by 8x (covers ~5-10 frames per read)
    // - Better SD card block alignment (512KB = typical erase block)
    // - Trade-off: Uses ~450KB more memory, acceptable for 24MB constraint
    const BUFFER_SIZE: usize = 524288;
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
    pub async fn new(file_path: &str, frame_rate: u32) -> Result<Self, H264FileError> {
        let mut file = File::open(file_path).await?;
        let file_size = file.seek(SeekFrom::End(0)).await?;
        file.seek(SeekFrom::Start(0)).await?;

        // Default to 40ms (25fps) when frame_rate is 0
        let frame_duration_ms = 1000u32.checked_div(frame_rate).unwrap_or(40);

        let nal_format = Self::detect_nal_format(&mut file, file_size).await?;

        Ok(Self {
            file,
            buffer: vec![0u8; Self::BUFFER_SIZE],
            annexb_data: Vec::with_capacity(Self::BUFFER_SIZE),
            annexb_start: 0,
            annexb_eof: false,
            current_pos: 0,
            file_size,
            frame_rate,
            frame_duration_ms,
            nal_format,
        })
    }

    async fn detect_nal_format(
        file: &mut File,
        file_size: u64,
    ) -> Result<NalFormat, H264FileError> {
        let detect_len = std::cmp::min(
            Self::BUFFER_SIZE + Self::START_CODE_4.len(),
            file_size as usize,
        );
        let mut header = vec![0u8; detect_len];
        let bytes_read = file.read(&mut header).await?;
        header.truncate(bytes_read);
        file.seek(SeekFrom::Start(0)).await?;

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
    /// `Option<NalUnit>` if successful, None at EOF, or H264FileError
    pub async fn read_next_nal(&mut self) -> Result<Option<NalUnit>, H264FileError> {
        match self.nal_format {
            NalFormat::AnnexB => self.read_next_nal_annexb().await,
            NalFormat::Avcc => self.read_next_nal_avcc().await,
        }
    }

    async fn read_next_nal_annexb(&mut self) -> Result<Option<NalUnit>, H264FileError> {
        loop {
            if self.annexb_start >= self.annexb_data.len() && !self.fill_annexb_buffer().await? {
                return Ok(None);
            }

            // Try to find start code at current position
            let (start_code_idx, start_code_len) =
                match self.find_start_code_at_current_pos().await? {
                    Some(found) => found,
                    None => continue,
                };

            // Skip to start code if not at current position
            if start_code_idx > self.annexb_start {
                self.consume_annexb_bytes(start_code_idx - self.annexb_start);
                continue;
            }

            // Try to extract NAL unit
            let nal_start = start_code_idx + start_code_len;
            if let Some(result) = self.try_extract_nal_unit(nal_start, start_code_len).await? {
                return Ok(Some(result));
            }
            // If None returned, continue loop to try again with more data
        }
    }

    /// Find start code at current position, filling buffer if needed
    async fn find_start_code_at_current_pos(
        &mut self,
    ) -> Result<Option<(usize, usize)>, H264FileError> {
        match Self::find_start_code(&self.annexb_data, self.annexb_start) {
            Some(found) => Ok(Some(found)),
            None => {
                if self.fill_annexb_buffer().await? {
                    return Ok(None); // Signal to continue loop
                }
                let remaining = self.annexb_data.len().saturating_sub(self.annexb_start);
                self.consume_annexb_bytes(remaining);
                Ok(None)
            }
        }
    }

    /// Try to extract a NAL unit starting at nal_start position
    async fn try_extract_nal_unit(
        &mut self,
        nal_start: usize,
        start_code_len: usize,
    ) -> Result<Option<NalUnit>, H264FileError> {
        // Look for next start code to determine NAL boundary
        if let Some((next_start_code_idx, _)) = Self::find_start_code(&self.annexb_data, nal_start)
        {
            return self
                .extract_nal_with_boundary(nal_start, next_start_code_idx, start_code_len)
                .await;
        }

        // No boundary found, try to fill buffer
        if self.fill_annexb_buffer().await? {
            return Ok(None); // Signal to continue loop
        }

        // EOF - extract remaining data as NAL
        self.extract_nal_at_eof(nal_start, start_code_len).await
    }

    /// Extract NAL unit when we found a boundary
    async fn extract_nal_with_boundary(
        &mut self,
        nal_start: usize,
        next_start_code_idx: usize,
        start_code_len: usize,
    ) -> Result<Option<NalUnit>, H264FileError> {
        // Handle case where next start code is immediately after current (empty NAL)
        if next_start_code_idx == nal_start {
            self.consume_annexb_bytes(start_code_len);
            return Ok(None); // Signal to continue loop
        }

        let nal_data = self.annexb_data[nal_start..next_start_code_idx].to_vec();
        self.consume_annexb_bytes(next_start_code_idx - self.annexb_start);
        Self::build_annexb_nal(nal_data, start_code_len).map(Some)
    }

    /// Extract NAL unit at EOF
    async fn extract_nal_at_eof(
        &mut self,
        nal_start: usize,
        start_code_len: usize,
    ) -> Result<Option<NalUnit>, H264FileError> {
        if nal_start >= self.annexb_data.len() {
            let remaining = self.annexb_data.len().saturating_sub(self.annexb_start);
            self.consume_annexb_bytes(remaining);
            return Ok(None);
        }

        let nal_data = self.annexb_data[nal_start..].to_vec();
        let remaining = self.annexb_data.len().saturating_sub(self.annexb_start);
        self.consume_annexb_bytes(remaining);
        Self::build_annexb_nal(nal_data, start_code_len).map(Some)
    }

    async fn read_next_nal_avcc(&mut self) -> Result<Option<NalUnit>, H264FileError> {
        if self.current_pos >= self.file_size {
            return Ok(None);
        }

        let nal_len = self.read_avcc_length().await?;
        if nal_len == 0 || self.current_pos + nal_len as u64 > self.file_size {
            return Err(H264FileError::InvalidNalUnit);
        }

        let nal_data = self.read_avcc_nal_data(nal_len).await?;
        if nal_data.is_empty() {
            return Err(H264FileError::InvalidNalUnit);
        }

        Self::validate_and_build_avcc_nal(nal_data).map(Some)
    }

    async fn read_avcc_length(&mut self) -> Result<usize, H264FileError> {
        let mut len_buf = [0u8; 4];
        let bytes_read = self.file.read(&mut len_buf).await?;
        if bytes_read < 4 {
            return Ok(0);
        }
        self.current_pos += 4;
        Ok(u32::from_be_bytes(len_buf) as usize)
    }

    async fn read_avcc_nal_data(&mut self, nal_len: usize) -> Result<Vec<u8>, H264FileError> {
        let mut nal_data = vec![0u8; nal_len];
        let data_read = self.file.read(&mut nal_data).await?;
        nal_data.truncate(data_read);
        self.current_pos += data_read as u64;
        Ok(nal_data)
    }

    fn validate_and_build_avcc_nal(nal_data: Vec<u8>) -> Result<NalUnit, H264FileError> {
        let forbidden_bit = (nal_data[0] >> 7) & 1;
        if forbidden_bit != 0 {
            return Err(H264FileError::InvalidNalUnit);
        }

        let nal_unit_type = nal_data[0] & 0x1f;
        Ok(NalUnit {
            unit_type: NalUnitType::from(nal_unit_type),
            data: nal_data,
            start_code_length: 0,
        })
    }

    fn build_annexb_nal(
        nal_data: Vec<u8>,
        start_code_length: usize,
    ) -> Result<NalUnit, H264FileError> {
        if nal_data.is_empty() {
            return Err(H264FileError::InvalidNalUnit);
        }

        let forbidden_bit = (nal_data[0] >> 7) & 1;
        if forbidden_bit != 0 {
            return Err(H264FileError::InvalidNalUnit);
        }

        let nal_unit_type = nal_data[0] & 0x1f;
        Ok(NalUnit {
            unit_type: NalUnitType::from(nal_unit_type),
            data: nal_data,
            start_code_length,
        })
    }

    async fn fill_annexb_buffer(&mut self) -> Result<bool, H264FileError> {
        if self.annexb_eof {
            return Ok(false);
        }

        let bytes_read = self.file.read(&mut self.buffer).await?;
        if bytes_read == 0 {
            self.annexb_eof = true;
            return Ok(false);
        }

        self.annexb_data
            .extend_from_slice(&self.buffer[..bytes_read]);
        Ok(true)
    }

    fn consume_annexb_bytes(&mut self, count: usize) {
        let available = self.annexb_data.len().saturating_sub(self.annexb_start);
        let consumed = count.min(available);
        self.annexb_start += consumed;
        self.current_pos = self.current_pos.saturating_add(consumed as u64);
        self.compact_annexb_buffer();
    }

    fn compact_annexb_buffer(&mut self) {
        if self.annexb_start == 0 {
            return;
        }
        if self.annexb_start >= self.annexb_data.len() {
            self.annexb_data.clear();
            self.annexb_start = 0;
            return;
        }
        if self.annexb_start >= Self::BUFFER_SIZE {
            self.annexb_data.drain(..self.annexb_start);
            self.annexb_start = 0;
        }
    }

    fn find_start_code(data: &[u8], from: usize) -> Option<(usize, usize)> {
        if data.len() < 3 || from >= data.len() {
            return None;
        }

        let mut i = from;
        while i + 2 < data.len() {
            if i + 3 < data.len()
                && data[i] == 0x00
                && data[i + 1] == 0x00
                && data[i + 2] == 0x00
                && data[i + 3] == 0x01
            {
                return Some((i, 4));
            }
            if data[i] == 0x00 && data[i + 1] == 0x00 && data[i + 2] == 0x01 {
                return Some((i, 3));
            }
            i += 1;
        }
        None
    }

    /// Extract SPS (Sequence Parameter Set) from next NAL units
    ///
    /// # Returns
    ///
    /// Tuple of (SPS bytes, PPS bytes) or error
    pub async fn extract_sps_pps(&mut self) -> Result<(Vec<u8>, Vec<u8>), H264FileError> {
        self.reset().await?;

        let scan_len = std::cmp::min(self.file_size as usize, 1024 * 1024);
        let mut scan_buf = vec![0u8; scan_len];
        let bytes_read = self.file.read(&mut scan_buf).await?;
        scan_buf.truncate(bytes_read);

        let result = Self::scan_sps_pps_from_buffer(&scan_buf);
        self.reset().await?;

        let (sps, pps) = match result {
            Some(found) => found,
            None => self.scan_sps_pps_fallback().await?,
        };

        self.reset().await?;

        Self::validate_sps_pps(sps, pps)
    }

    async fn scan_sps_pps_fallback(&mut self) -> Result<(Vec<u8>, Vec<u8>), H264FileError> {
        let mut sps = Vec::new();
        let mut pps = Vec::new();

        while let Some(nal) = self.read_next_nal().await? {
            match nal.unit_type {
                NalUnitType::SequenceParameterSet if nal.data.len() <= Self::MAX_PARAM_SET_LEN => {
                    sps = nal.data.clone();
                }
                NalUnitType::PictureParameterSet if nal.data.len() <= Self::MAX_PARAM_SET_LEN => {
                    pps = nal.data.clone();
                }
                NalUnitType::IdrSlice => break,
                _ => {}
            }

            if !sps.is_empty() && !pps.is_empty() {
                break;
            }
        }

        Ok((sps, pps))
    }

    fn validate_sps_pps(sps: Vec<u8>, pps: Vec<u8>) -> Result<(Vec<u8>, Vec<u8>), H264FileError> {
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
        let start_codes = Self::collect_annexb_start_codes(buffer);
        Self::extract_sps_pps_from_nals(buffer, &start_codes)
    }

    fn collect_annexb_start_codes(buffer: &[u8]) -> Vec<(usize, usize)> {
        let mut start_codes: Vec<(usize, usize)> = Vec::new();
        let mut i = 0;

        while i + 3 < buffer.len() {
            if let Some(sc_len) = Self::match_start_code_at(buffer, i) {
                start_codes.push((i, sc_len));
                i += sc_len;
            } else {
                i += 1;
            }
        }
        start_codes
    }

    fn match_start_code_at(buffer: &[u8], pos: usize) -> Option<usize> {
        if pos + 4 <= buffer.len() && buffer[pos..pos + 4] == *Self::START_CODE_4 {
            return Some(4);
        }
        if buffer[pos..pos + 3] == *Self::START_CODE_3 {
            return Some(3);
        }
        None
    }

    fn extract_sps_pps_from_nals(
        buffer: &[u8],
        start_codes: &[(usize, usize)],
    ) -> Option<(Vec<u8>, Vec<u8>)> {
        let mut sps: Option<Vec<u8>> = None;
        let mut pps: Option<Vec<u8>> = None;

        for (idx, &(start_pos, sc_len)) in start_codes.iter().enumerate() {
            let nal_range = Self::get_nal_range(buffer, start_codes, idx, start_pos, sc_len);
            let Some((nal_start, nal_end)) = nal_range else {
                continue;
            };

            if let Some(nal_data) = Self::try_get_sps_or_pps(buffer, nal_start, nal_end) {
                match Self::nal_type(&nal_data) {
                    7 => sps = Some(nal_data),
                    8 => pps = Some(nal_data),
                    _ => {}
                }
            }

            if sps.is_some() && pps.is_some() {
                break;
            }
        }

        sps.zip(pps)
    }

    fn get_nal_range(
        buffer: &[u8],
        start_codes: &[(usize, usize)],
        idx: usize,
        start_pos: usize,
        sc_len: usize,
    ) -> Option<(usize, usize)> {
        let data_start = start_pos + sc_len;
        let data_end = start_codes
            .get(idx + 1)
            .map(|(pos, _)| *pos)
            .unwrap_or(buffer.len());

        (data_start < data_end).then_some((data_start, data_end))
    }

    fn try_get_sps_or_pps(buffer: &[u8], start: usize, end: usize) -> Option<Vec<u8>> {
        let nal = &buffer[start..end];
        (!nal.is_empty() && nal.len() <= Self::MAX_PARAM_SET_LEN).then(|| nal.to_vec())
    }

    fn nal_type(data: &[u8]) -> u8 {
        data.first().map(|b| b & 0x1f).unwrap_or(0)
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

            let Some(nal_data) = Self::try_get_sps_or_pps_from_avcc(nal) else {
                continue;
            };

            match Self::nal_type(&nal_data) {
                7 => sps = Some(nal_data),
                8 => pps = Some(nal_data),
                _ => {}
            }

            if sps.is_some() && pps.is_some() {
                break;
            }
        }

        sps.zip(pps)
    }

    fn try_get_sps_or_pps_from_avcc(nal: &[u8]) -> Option<Vec<u8>> {
        (!nal.is_empty() && nal.len() <= Self::MAX_PARAM_SET_LEN).then(|| nal.to_vec())
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
    pub async fn reset(&mut self) -> Result<(), H264FileError> {
        self.file.seek(SeekFrom::Start(0)).await?;
        self.current_pos = 0;
        self.annexb_data.clear();
        self.annexb_start = 0;
        self.annexb_eof = false;
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
    use std::env;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_temp_h264_file(test_name: &str, data: &[u8]) -> std::path::PathBuf {
        let temp_dir = env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file_path = temp_dir.join(format!("{test_name}_{nanos}.h264"));
        let mut temp_file = std::fs::File::create(&file_path).unwrap();
        temp_file.write_all(data).unwrap();
        drop(temp_file);
        file_path
    }

    #[test]
    fn test_nal_unit_type_conversion() {
        assert_eq!(NalUnitType::from(1), NalUnitType::NonIdrSlice);
        assert_eq!(NalUnitType::from(5), NalUnitType::IdrSlice);
        assert_eq!(NalUnitType::from(7), NalUnitType::SequenceParameterSet);
        assert_eq!(NalUnitType::from(8), NalUnitType::PictureParameterSet);
    }

    #[tokio::test]
    async fn test_frame_duration_calculation() {
        let reader = H264FileReader::new("/dev/null", 25).await.unwrap();
        assert_eq!(reader.frame_duration_ms(), 40);

        let reader = H264FileReader::new("/dev/null", 30).await.unwrap();
        assert_eq!(reader.frame_duration_ms(), 33);

        let reader = H264FileReader::new("/dev/null", 0).await.unwrap();
        assert_eq!(reader.frame_duration_ms(), 40); // Default fallback
    }

    #[tokio::test]
    async fn test_read_next_nal_start_code_across_buffer_boundary() {
        // Use non-zero padding to avoid accidental 4-byte start-code matches in filler bytes.
        let mut data = vec![0xFFu8; H264FileReader::BUFFER_SIZE - 2];
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

        let mut reader = H264FileReader::new(file_path.to_str().unwrap(), 25)
            .await
            .unwrap();
        let sps = reader.read_next_nal().await.unwrap().unwrap();
        assert_eq!(sps.unit_type, NalUnitType::SequenceParameterSet);
        assert_eq!(sps.data, vec![0x67, 0x42, 0x00, 0x1e]);

        let pps = reader.read_next_nal().await.unwrap().unwrap();
        assert_eq!(pps.unit_type, NalUnitType::PictureParameterSet);
        assert_eq!(pps.data, vec![0x68, 0xce, 0x06, 0xe2]);

        let _ = std::fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_next_nal_avcc_format() {
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

        let mut reader = H264FileReader::new(file_path.to_str().unwrap(), 25)
            .await
            .unwrap();
        let sps_nal = reader.read_next_nal().await.unwrap().unwrap();
        assert_eq!(sps_nal.unit_type, NalUnitType::SequenceParameterSet);
        assert_eq!(sps_nal.data, sps);

        let pps_nal = reader.read_next_nal().await.unwrap().unwrap();
        assert_eq!(pps_nal.unit_type, NalUnitType::PictureParameterSet);
        assert_eq!(pps_nal.data, pps);

        let _ = std::fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_next_nal_avcc_forbidden_bit_set_returns_invalid_nal() {
        // Keep payload free of Annex-B start-code signatures so format detection stays AVCC.
        let invalid_nal = [0x80, 0xAA, 0xBB, 0xCC];
        let mut data = Vec::new();
        data.extend_from_slice(&(invalid_nal.len() as u32).to_be_bytes());
        data.extend_from_slice(&invalid_nal);

        let file_path = create_temp_h264_file("h264_avcc_forbidden_bit", &data);

        let mut reader = H264FileReader::new(file_path.to_str().unwrap(), 25)
            .await
            .unwrap();

        let result = reader.read_next_nal().await;
        assert!(matches!(result, Err(H264FileError::InvalidNalUnit)));

        let _ = std::fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_next_nal_avcc_length_overflow_returns_invalid_nal() {
        // Declared NAL length (16) exceeds remaining file size (2).
        let data = [0x00, 0x00, 0x00, 0x10, 0x67, 0x42];
        let file_path = create_temp_h264_file("h264_avcc_length_overflow", &data);

        let mut reader = H264FileReader::new(file_path.to_str().unwrap(), 25)
            .await
            .unwrap();

        let result = reader.read_next_nal().await;
        assert!(matches!(result, Err(H264FileError::InvalidNalUnit)));

        let _ = std::fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_extract_sps_pps_without_parameter_sets_returns_no_nal_units() {
        // Annex-B stream with only an IDR NAL, no SPS/PPS.
        let data = [0x00, 0x00, 0x01, 0x65, 0xAA, 0xBB, 0xCC];
        let file_path = create_temp_h264_file("h264_extract_sps_pps_no_param_sets", &data);

        let mut reader = H264FileReader::new(file_path.to_str().unwrap(), 25)
            .await
            .unwrap();

        let result = reader.extract_sps_pps().await;
        assert!(matches!(result, Err(H264FileError::NoNalUnits)));

        let _ = std::fs::remove_file(file_path);
    }

    // ========== NalUnitType::from comprehensive tests ==========

    #[test]
    fn test_nal_unit_type_from_all_values() {
        assert_eq!(NalUnitType::from(0), NalUnitType::UnspecifiedHighValue);
        assert_eq!(NalUnitType::from(2), NalUnitType::PartitionASlice);
        assert_eq!(NalUnitType::from(3), NalUnitType::PartitionBSlice);
        assert_eq!(NalUnitType::from(4), NalUnitType::PartitionCSlice);
        assert_eq!(
            NalUnitType::from(6),
            NalUnitType::SupplementalEnhancementInformation
        );
        assert_eq!(NalUnitType::from(9), NalUnitType::AccessUnitDelimiter);
        assert_eq!(NalUnitType::from(10), NalUnitType::EndOfSequence);
        assert_eq!(NalUnitType::from(11), NalUnitType::EndOfStream);
        assert_eq!(NalUnitType::from(12), NalUnitType::FillerData);
        assert_eq!(
            NalUnitType::from(13),
            NalUnitType::SequenceParameterSetExtension
        );
        assert_eq!(NalUnitType::from(14), NalUnitType::PrefixNalUnit);
        assert_eq!(
            NalUnitType::from(15),
            NalUnitType::SubsetSequenceParameterSet
        );
        assert_eq!(NalUnitType::from(16), NalUnitType::DepthParameterSet);
    }

    #[test]
    fn test_nal_unit_type_from_reserved_ranges() {
        assert_eq!(NalUnitType::from(17), NalUnitType::Reserved);
        assert_eq!(NalUnitType::from(18), NalUnitType::Reserved);
        assert_eq!(NalUnitType::from(19), NalUnitType::Reserved);
        assert_eq!(NalUnitType::from(23), NalUnitType::Reserved);
    }

    #[test]
    fn test_nal_unit_type_from_unspecified_high() {
        assert_eq!(NalUnitType::from(24), NalUnitType::UnspecifiedHighValue);
        assert_eq!(NalUnitType::from(31), NalUnitType::UnspecifiedHighValue);
        assert_eq!(NalUnitType::from(255), NalUnitType::UnspecifiedHighValue);
    }

    // ========== find_start_code tests ==========

    #[test]
    fn test_find_start_code_3byte() {
        let data = [0x00, 0x00, 0x01, 0x67];
        assert_eq!(H264FileReader::find_start_code(&data, 0), Some((0, 3)));
    }

    #[test]
    fn test_find_start_code_4byte() {
        let data = [0x00, 0x00, 0x00, 0x01, 0x67];
        assert_eq!(H264FileReader::find_start_code(&data, 0), Some((0, 4)));
    }

    #[test]
    fn test_find_start_code_with_offset() {
        let data = [0xAA, 0xBB, 0x00, 0x00, 0x01, 0x67];
        assert_eq!(H264FileReader::find_start_code(&data, 0), Some((2, 3)));
        assert_eq!(H264FileReader::find_start_code(&data, 2), Some((2, 3)));
        assert_eq!(H264FileReader::find_start_code(&data, 3), None);
    }

    #[test]
    fn test_find_start_code_no_match() {
        let data = [0xAA, 0xBB, 0xCC, 0xDD];
        assert_eq!(H264FileReader::find_start_code(&data, 0), None);
    }

    #[test]
    fn test_find_start_code_empty() {
        assert_eq!(H264FileReader::find_start_code(&[], 0), None);
    }

    #[test]
    fn test_find_start_code_too_short() {
        assert_eq!(H264FileReader::find_start_code(&[0x00, 0x00], 0), None);
    }

    #[test]
    fn test_find_start_code_from_past_end() {
        let data = [0x00, 0x00, 0x01, 0x67];
        assert_eq!(H264FileReader::find_start_code(&data, 10), None);
    }

    #[test]
    fn test_find_start_code_prefers_4byte_over_3byte() {
        // 4-byte start code: 00 00 00 01
        let data = [0x00, 0x00, 0x00, 0x01, 0x67];
        let result = H264FileReader::find_start_code(&data, 0);
        assert_eq!(result, Some((0, 4)));
    }

    #[test]
    fn test_find_start_code_multiple() {
        // Two NALs with 3-byte start codes
        let data = [0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x00, 0x01, 0x68];
        assert_eq!(H264FileReader::find_start_code(&data, 0), Some((0, 3)));
        assert_eq!(H264FileReader::find_start_code(&data, 3), Some((5, 3)));
    }

    // ========== build_annexb_nal tests ==========

    #[test]
    fn test_build_annexb_nal_valid_sps() {
        let nal_data = vec![0x67, 0x42, 0x00, 0x1E]; // SPS NAL
        let result = H264FileReader::build_annexb_nal(nal_data.clone(), 3);
        assert!(result.is_ok());
        let nal = result.unwrap();
        assert_eq!(nal.unit_type, NalUnitType::SequenceParameterSet);
        assert_eq!(nal.data, nal_data);
        assert_eq!(nal.start_code_length, 3);
    }

    #[test]
    fn test_build_annexb_nal_valid_idr() {
        let nal_data = vec![0x65, 0xAA, 0xBB]; // IDR NAL
        let result = H264FileReader::build_annexb_nal(nal_data, 4);
        assert!(result.is_ok());
        let nal = result.unwrap();
        assert_eq!(nal.unit_type, NalUnitType::IdrSlice);
        assert_eq!(nal.start_code_length, 4);
    }

    #[test]
    fn test_build_annexb_nal_empty_data() {
        let result = H264FileReader::build_annexb_nal(vec![], 3);
        assert!(matches!(result, Err(H264FileError::InvalidNalUnit)));
    }

    #[test]
    fn test_build_annexb_nal_forbidden_bit_set() {
        let nal_data = vec![0x80, 0x42]; // forbidden_zero_bit = 1
        let result = H264FileReader::build_annexb_nal(nal_data, 3);
        assert!(matches!(result, Err(H264FileError::InvalidNalUnit)));
    }

    // ========== validate_and_build_avcc_nal tests ==========

    #[test]
    fn test_validate_and_build_avcc_nal_valid() {
        let nal_data = vec![0x67, 0x42, 0x00, 0x1E]; // SPS
        let result = H264FileReader::validate_and_build_avcc_nal(nal_data);
        assert!(result.is_ok());
        let nal = result.unwrap();
        assert_eq!(nal.unit_type, NalUnitType::SequenceParameterSet);
        assert_eq!(nal.start_code_length, 0);
    }

    #[test]
    fn test_validate_and_build_avcc_nal_forbidden_bit() {
        let nal_data = vec![0x87, 0x42]; // forbidden_zero_bit = 1
        let result = H264FileReader::validate_and_build_avcc_nal(nal_data);
        assert!(matches!(result, Err(H264FileError::InvalidNalUnit)));
    }

    // ========== validate_sps_pps tests ==========

    #[test]
    fn test_validate_sps_pps_both_valid() {
        let result = H264FileReader::validate_sps_pps(vec![0x67, 0x42], vec![0x68, 0xCE]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_sps_pps_empty_sps() {
        let result = H264FileReader::validate_sps_pps(vec![], vec![0x68, 0xCE]);
        assert!(matches!(result, Err(H264FileError::NoNalUnits)));
    }

    #[test]
    fn test_validate_sps_pps_empty_pps() {
        let result = H264FileReader::validate_sps_pps(vec![0x67, 0x42], vec![]);
        assert!(matches!(result, Err(H264FileError::NoNalUnits)));
    }

    #[test]
    fn test_validate_sps_pps_both_empty() {
        let result = H264FileReader::validate_sps_pps(vec![], vec![]);
        assert!(matches!(result, Err(H264FileError::NoNalUnits)));
    }

    // ========== match_start_code_at tests ==========

    #[test]
    fn test_match_start_code_at_4byte() {
        let data = [0x00, 0x00, 0x00, 0x01, 0x67];
        assert_eq!(H264FileReader::match_start_code_at(&data, 0), Some(4));
    }

    #[test]
    fn test_match_start_code_at_3byte() {
        let data = [0x00, 0x00, 0x01, 0x67];
        assert_eq!(H264FileReader::match_start_code_at(&data, 0), Some(3));
    }

    #[test]
    fn test_match_start_code_at_no_match() {
        let data = [0xAA, 0xBB, 0xCC, 0xDD];
        assert_eq!(H264FileReader::match_start_code_at(&data, 0), None);
    }

    // ========== nal_type tests ==========

    #[test]
    fn test_nal_type_sps() {
        assert_eq!(H264FileReader::nal_type(&[0x67]), 7); // SPS
    }

    #[test]
    fn test_nal_type_pps() {
        assert_eq!(H264FileReader::nal_type(&[0x68]), 8); // PPS
    }

    #[test]
    fn test_nal_type_idr() {
        assert_eq!(H264FileReader::nal_type(&[0x65]), 5); // IDR
    }

    #[test]
    fn test_nal_type_empty() {
        assert_eq!(H264FileReader::nal_type(&[]), 0);
    }

    // ========== collect_annexb_start_codes tests ==========

    #[test]
    fn test_collect_annexb_start_codes_multiple() {
        let data = [
            0x00, 0x00, 0x01, 0x67, 0x42, // 3-byte SPS
            0x00, 0x00, 0x00, 0x01, 0x68, 0xCE, // 4-byte PPS
        ];
        let codes = H264FileReader::collect_annexb_start_codes(&data);
        assert_eq!(codes.len(), 2);
        assert_eq!(codes[0], (0, 3));
        assert_eq!(codes[1], (5, 4));
    }

    #[test]
    fn test_collect_annexb_start_codes_empty() {
        let codes = H264FileReader::collect_annexb_start_codes(&[]);
        assert!(codes.is_empty());
    }

    #[test]
    fn test_collect_annexb_start_codes_no_codes() {
        let data = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE];
        let codes = H264FileReader::collect_annexb_start_codes(&data);
        assert!(codes.is_empty());
    }

    // ========== scan_annexb_for_sps_pps tests ==========

    #[test]
    fn test_scan_annexb_for_sps_pps_found() {
        let data = [
            0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1E, // SPS
            0x00, 0x00, 0x01, 0x68, 0xCE, 0x06, 0xE2, // PPS
        ];
        let result = H264FileReader::scan_annexb_for_sps_pps(&data);
        assert!(result.is_some());
        let (sps, pps) = result.unwrap();
        assert_eq!(sps[0] & 0x1F, 7); // SPS type
        assert_eq!(pps[0] & 0x1F, 8); // PPS type
    }

    #[test]
    fn test_scan_annexb_for_sps_pps_missing_pps() {
        let data = [
            0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1E, // SPS only
        ];
        let result = H264FileReader::scan_annexb_for_sps_pps(&data);
        assert!(result.is_none());
    }

    #[test]
    fn test_scan_annexb_for_sps_pps_missing_sps() {
        let data = [
            0x00, 0x00, 0x01, 0x68, 0xCE, 0x06, 0xE2, // PPS only
        ];
        let result = H264FileReader::scan_annexb_for_sps_pps(&data);
        assert!(result.is_none());
    }

    // ========== scan_avcc_for_sps_pps tests ==========

    #[test]
    fn test_scan_avcc_for_sps_pps_found() {
        let sps = [0x67, 0x42, 0x00, 0x1E];
        let pps = [0x68, 0xCE, 0x06, 0xE2];
        let mut data = Vec::new();
        data.extend_from_slice(&(sps.len() as u32).to_be_bytes());
        data.extend_from_slice(&sps);
        data.extend_from_slice(&(pps.len() as u32).to_be_bytes());
        data.extend_from_slice(&pps);

        let result = H264FileReader::scan_avcc_for_sps_pps(&data);
        assert!(result.is_some());
        let (found_sps, found_pps) = result.unwrap();
        assert_eq!(found_sps, sps);
        assert_eq!(found_pps, pps);
    }

    #[test]
    fn test_scan_avcc_for_sps_pps_zero_length_nal() {
        // Length = 0 should stop scanning
        let data = [0x00, 0x00, 0x00, 0x00];
        let result = H264FileReader::scan_avcc_for_sps_pps(&data);
        assert!(result.is_none());
    }

    #[test]
    fn test_scan_avcc_for_sps_pps_empty() {
        let result = H264FileReader::scan_avcc_for_sps_pps(&[]);
        assert!(result.is_none());
    }

    // ========== H264FileError display tests ==========

    #[test]
    fn test_h264_file_error_display() {
        let err = H264FileError::InvalidFormat;
        assert_eq!(format!("{err}"), "Invalid H264 file format");

        let err = H264FileError::NoNalUnits;
        assert_eq!(format!("{err}"), "No NAL units found");

        let err = H264FileError::InvalidNalUnit;
        assert_eq!(format!("{err}"), "Invalid NAL unit");
    }

    #[test]
    fn test_h264_file_error_io_error_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let h264_err: H264FileError = io_err.into();
        assert!(format!("{h264_err}").contains("file not found"));
    }

    // ========== NalUnit struct tests ==========

    #[test]
    fn test_nal_unit_clone() {
        let nal = NalUnit {
            unit_type: NalUnitType::SequenceParameterSet,
            data: vec![0x67, 0x42],
            start_code_length: 4,
        };
        let cloned = nal.clone();
        assert_eq!(cloned.unit_type, NalUnitType::SequenceParameterSet);
        assert_eq!(cloned.data, vec![0x67, 0x42]);
        assert_eq!(cloned.start_code_length, 4);
    }

    #[test]
    fn test_nal_unit_debug() {
        let nal = NalUnit {
            unit_type: NalUnitType::IdrSlice,
            data: vec![0x65],
            start_code_length: 3,
        };
        let debug = format!("{nal:?}");
        assert!(debug.contains("IdrSlice"));
    }

    // ========== NalFormat equality tests ==========

    #[test]
    fn test_nal_format_eq() {
        assert_eq!(NalFormat::AnnexB, NalFormat::AnnexB);
        assert_eq!(NalFormat::Avcc, NalFormat::Avcc);
        assert_ne!(NalFormat::AnnexB, NalFormat::Avcc);
    }

    // ========== Async file-based tests ==========

    #[tokio::test]
    async fn test_reader_file_size_and_position() {
        let data = [0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1E];
        let file_path = create_temp_h264_file("h264_size_pos", &data);

        let reader = H264FileReader::new(file_path.to_str().unwrap(), 25)
            .await
            .unwrap();
        assert_eq!(reader.file_size(), data.len() as u64);
        assert_eq!(reader.current_position(), 0);
        assert_eq!(reader.frame_rate(), 25);

        let _ = std::fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_reader_reset() {
        let data = [
            0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1E, // SPS
            0x00, 0x00, 0x01, 0x68, 0xCE, 0x06, 0xE2, // PPS
        ];
        let file_path = create_temp_h264_file("h264_reset", &data);

        let mut reader = H264FileReader::new(file_path.to_str().unwrap(), 25)
            .await
            .unwrap();

        // Read first NAL
        let _ = reader.read_next_nal().await.unwrap();
        // Reset and read first NAL again
        reader.reset().await.unwrap();
        assert_eq!(reader.current_position(), 0);

        let nal = reader.read_next_nal().await.unwrap().unwrap();
        assert_eq!(nal.unit_type, NalUnitType::SequenceParameterSet);

        let _ = std::fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_extract_sps_pps_annexb_success() {
        let data = [
            0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1E, // SPS
            0x00, 0x00, 0x01, 0x68, 0xCE, 0x06, 0xE2, // PPS
        ];
        let file_path = create_temp_h264_file("h264_sps_pps_ok", &data);

        let mut reader = H264FileReader::new(file_path.to_str().unwrap(), 25)
            .await
            .unwrap();

        let (sps, pps) = reader.extract_sps_pps().await.unwrap();
        assert_eq!(sps[0] & 0x1F, 7); // SPS type
        assert_eq!(pps[0] & 0x1F, 8); // PPS type

        let _ = std::fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_empty_file() {
        let file_path = create_temp_h264_file("h264_empty", &[]);

        let mut reader = H264FileReader::new(file_path.to_str().unwrap(), 25)
            .await
            .unwrap();
        let result = reader.read_next_nal().await.unwrap();
        assert!(result.is_none());

        let _ = std::fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_read_annexb_multiple_nals() {
        let data = [
            0x00, 0x00, 0x00, 0x01, 0x67, 0x42, // SPS (4-byte start code)
            0x00, 0x00, 0x01, 0x68, 0xCE, // PPS (3-byte start code)
            0x00, 0x00, 0x01, 0x65, 0xAA, 0xBB, // IDR
        ];
        let file_path = create_temp_h264_file("h264_multi_nal", &data);

        let mut reader = H264FileReader::new(file_path.to_str().unwrap(), 25)
            .await
            .unwrap();

        let nal1 = reader.read_next_nal().await.unwrap().unwrap();
        assert_eq!(nal1.unit_type, NalUnitType::SequenceParameterSet);

        let nal2 = reader.read_next_nal().await.unwrap().unwrap();
        assert_eq!(nal2.unit_type, NalUnitType::PictureParameterSet);

        let nal3 = reader.read_next_nal().await.unwrap().unwrap();
        assert_eq!(nal3.unit_type, NalUnitType::IdrSlice);

        let nal4 = reader.read_next_nal().await.unwrap();
        assert!(nal4.is_none());

        let _ = std::fs::remove_file(file_path);
    }
}
