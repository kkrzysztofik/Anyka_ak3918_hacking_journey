use {
    super::{
        bits_errors::{BitError, BitErrorValue},
        bytes_writer::BytesWriter,
    },
    bytes::BytesMut,
};

pub struct BitsWriter {
    writer: BytesWriter,
    cur_byte: u8,
    cur_bit_num: u8,
}

impl BitsWriter {
    pub fn new(writer: BytesWriter) -> Self {
        Self {
            writer,
            cur_byte: 0,
            cur_bit_num: 0,
        }
    }

    pub fn write_bytes(&mut self, data: BytesMut) -> Result<(), BitError> {
        // Flush any pending bits before writing bytes to maintain correct byte alignment
        if self.cur_bit_num != 0 {
            self.flush()?;
        }
        self.writer.write(&data[..])?;
        Ok(())
    }

    pub fn write_bit(&mut self, b: u8) -> Result<(), BitError> {
        // Validate that b is a single bit (0 or 1)
        if b & !0x01 != 0 {
            return Err(BitError {
                value: BitErrorValue::InvalidBitValue,
            });
        }

        self.cur_byte |= b << (7 - self.cur_bit_num);
        self.cur_bit_num += 1;

        if self.cur_bit_num == 8 {
            self.writer.write_u8(self.cur_byte)?;
            self.cur_bit_num = 0;
            self.cur_byte = 0;
        }

        Ok(())
    }

    pub fn write_8bit(&mut self, b: u8) -> Result<(), BitError> {
        if self.cur_bit_num != 0 {
            return Err(BitError {
                value: BitErrorValue::CannotWrite8Bit,
            });
        }

        self.writer.write_u8(b)?;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), BitError> {
        if self.cur_bit_num > 0 {
            self.writer.write_u8(self.cur_byte)?;
            self.cur_bit_num = 0;
            self.cur_byte = 0;
        }

        Ok(())
    }

    // 0x02 4
    pub fn write_n_bits(&mut self, data: u64, bit_num: usize) -> Result<(), BitError> {
        if bit_num == 0 {
            return Ok(()); // No bits to write
        }
        if bit_num > 64 {
            return Err(BitError {
                value: BitErrorValue::TooBig,
            });
        }
        let mut bit_num_mut = bit_num;
        let mut data_mut = data;

        //read left bits  for current byte
        data_mut <<= 64 - bit_num;
        self.cur_byte |= (data_mut >> (56 + self.cur_bit_num)) as u8;

        let cur_byte_left_bit_num = 8 - self.cur_bit_num as usize;
        if bit_num_mut >= cur_byte_left_bit_num {
            // the bits for current byte is full, then flush
            data_mut <<= cur_byte_left_bit_num;
            bit_num_mut -= cur_byte_left_bit_num;
            self.cur_bit_num = 8;
            self.flush()?;
        } else {
            // not full, only update bit num
            self.cur_bit_num += bit_num_mut as u8;
            return Ok(());
        }

        while bit_num_mut > 0 {
            self.cur_byte = (data_mut >> 56) as u8;

            if bit_num_mut >= 8 {
                self.cur_bit_num = 8;
                self.flush()?;
                data_mut <<= 8;
                bit_num_mut -= 8;
            } else {
                self.cur_bit_num = bit_num_mut as u8;
                break;
            }
        }

        Ok(())
    }

    pub fn bits_alignment_8(&mut self) -> Result<(), BitError> {
        // If we have partial bits, flush them (already zero-padded implicitly)
        if self.cur_bit_num > 0 {
            self.flush()?;
        }
        Ok(())
    }

    /// Deprecated: use bits_alignment_8 instead (corrected spelling)
    #[deprecated(since = "0.2.0", note = "use bits_alignment_8 instead")]
    pub fn bits_aligment_8(&mut self) -> Result<(), BitError> {
        self.bits_alignment_8()
    }

    pub fn get_current_bytes(&self) -> BytesMut {
        self.writer.get_current_bytes()
    }

    pub fn len(&self) -> usize {
        self.writer.len() * 8 + self.cur_bit_num as usize
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    // ============================================
    // BitsWriter Construction and Basic Operations
    // ============================================

    #[test]
    fn test_bits_writer_new() {
        let bytes_writer = BytesWriter::new();
        let bits_writer = BitsWriter::new(bytes_writer);
        assert_eq!(bits_writer.len(), 0);
        assert!(bits_writer.is_empty());
    }

    // ============================================
    // write_bit Tests
    // ============================================

    #[test]
    fn test_bits_writer_write_bit_single_byte_zeros() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        for _ in 0..8 {
            bits_writer.write_bit(0).unwrap();
        }

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0x00]);
    }

    #[test]
    fn test_bits_writer_write_bit_single_byte_ones() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        for _ in 0..8 {
            bits_writer.write_bit(1).unwrap();
        }

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0xFF]);
    }

    #[test]
    fn test_bits_writer_write_bit_alternating() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // Write 10101010
        for i in 0..8 {
            bits_writer
                .write_bit(if i % 2 == 0 { 1 } else { 0 })
                .unwrap();
        }

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0xAA]);
    }

    #[test]
    fn test_bits_writer_write_bit_original() {
        // Original test preserved
        let bytes_writer = BytesWriter::new();
        let mut bit_writer = BitsWriter::new(bytes_writer);

        bit_writer.write_bit(0).unwrap();
        bit_writer.write_bit(0).unwrap();
        bit_writer.write_bit(0).unwrap();
        bit_writer.write_bit(0).unwrap();

        bit_writer.write_bit(0).unwrap();
        bit_writer.write_bit(0).unwrap();
        bit_writer.write_bit(1).unwrap();
        bit_writer.write_bit(0).unwrap();

        let byte = bit_writer.get_current_bytes();
        assert!(byte.to_vec()[0] == 0x2);

        bit_writer.write_bit(1).unwrap();
        bit_writer.write_bit(1).unwrap();

        assert!(bit_writer.cur_bit_num == 2);
        assert!(bit_writer.cur_byte == 0xC0); //0x11000000
    }

    #[test]
    fn test_bits_writer_write_bit_multi_byte() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // Write 0xFF 0x00 (16 bits)
        for _ in 0..8 {
            bits_writer.write_bit(1).unwrap();
        }
        for _ in 0..8 {
            bits_writer.write_bit(0).unwrap();
        }

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0xFF, 0x00]);
    }

    #[test]
    fn test_bits_writer_write_bit_partial_byte() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // Write only 4 bits (1111)
        for _ in 0..4 {
            bits_writer.write_bit(1).unwrap();
        }

        // Should have 4 bits pending
        assert_eq!(bits_writer.len(), 4);
        // Current byte should be 0xF0 (1111 0000)
        assert_eq!(bits_writer.cur_byte, 0xF0);
        assert_eq!(bits_writer.cur_bit_num, 4);
    }

    // ============================================
    // write_8bit Tests
    // ============================================

    #[test]
    fn test_bits_writer_write_8bit_success() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        bits_writer.write_8bit(0x42).unwrap();
        bits_writer.write_8bit(0x43).unwrap();

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0x42, 0x43]);
    }

    #[test]
    fn test_bits_writer_write_8bit_boundary_values() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        bits_writer.write_8bit(0x00).unwrap();
        bits_writer.write_8bit(0x7F).unwrap();
        bits_writer.write_8bit(0x80).unwrap();
        bits_writer.write_8bit(0xFF).unwrap();

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0x00, 0x7F, 0x80, 0xFF]);
    }

    #[test]
    fn test_bits_writer_write_8bit_after_partial_bits_error() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // Write some bits first
        bits_writer.write_bit(1).unwrap();

        // Now try to write 8 bits - should fail
        let result = bits_writer.write_8bit(0x42);
        assert!(result.is_err());
        match result.unwrap_err().value {
            BitErrorValue::CannotWrite8Bit => {}
            _ => panic!("Expected CannotWrite8Bit error"),
        }
    }

    // ============================================
    // write_n_bits Tests
    // ============================================

    #[test]
    fn test_bits_writer_write_n_bits_1_bit() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        bits_writer.write_n_bits(1, 1).unwrap();

        assert_eq!(bits_writer.cur_byte, 0x80); // 10000000
        assert_eq!(bits_writer.cur_bit_num, 1);
    }

    #[test]
    fn test_bits_writer_write_n_bits_4_bits() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        bits_writer.write_n_bits(0x0F, 4).unwrap();

        assert_eq!(bits_writer.cur_byte, 0xF0); // 11110000
        assert_eq!(bits_writer.cur_bit_num, 4);
    }

    #[test]
    fn test_bits_writer_write_n_bits_8_bits() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        bits_writer.write_n_bits(0xAB, 8).unwrap();

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0xAB]);
    }

    #[test]
    fn test_bits_writer_write_n_bits_16_bits() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        bits_writer.write_n_bits(0x1234, 16).unwrap();

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0x12, 0x34]);
    }

    #[test]
    fn test_bits_writer_write_n_bits_24_bits() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        bits_writer.write_n_bits(0x123456, 24).unwrap();

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0x12, 0x34, 0x56]);
    }

    #[test]
    fn test_bits_writer_write_n_bits_32_bits() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        bits_writer.write_n_bits(0x12345678, 32).unwrap();

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn test_bits_writer_write_n_bits_original() {
        // Original test preserved
        let bytes_writer = BytesWriter::new();
        let mut bit_writer = BitsWriter::new(bytes_writer);

        bit_writer.write_bit(1).unwrap();
        bit_writer.write_bit(1).unwrap();
        bit_writer.write_bit(0).unwrap();

        bit_writer.write_n_bits(0x03, 7).unwrap();

        let byte = bit_writer.get_current_bytes();

        assert!(byte.to_vec()[0] == 0xC0); //0x11000000

        assert!(bit_writer.cur_bit_num == 2);
        assert!(bit_writer.cur_byte == 0xC0); //0x11000000
    }

    #[test]
    fn test_bits_writer_write_n_bits_cross_byte_boundary() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // Write 4 bits, then 12 bits crossing byte boundary
        bits_writer.write_n_bits(0x0F, 4).unwrap();
        bits_writer.write_n_bits(0xABC, 12).unwrap();

        let bytes = bits_writer.get_current_bytes();
        // 0x0F (4 bits) + 0xABC (12 bits) = 0xFABC (16 bits) = 0xFA, 0xBC
        assert_eq!(bytes.to_vec(), vec![0xFA, 0xBC]);
    }

    #[test]
    fn test_bits_writer_write_n_bits_too_big_error() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        let result = bits_writer.write_n_bits(0, 65);
        assert!(result.is_err());
        match result.unwrap_err().value {
            BitErrorValue::TooBig => {}
            _ => panic!("Expected TooBig error"),
        }
    }

    #[test]
    fn test_bits_writer_write_n_bits_zero_bits() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        bits_writer.write_n_bits(0xFFFF, 0).unwrap();

        // Nothing should be written
        assert!(bits_writer.is_empty());
    }

    #[test]
    fn test_bits_writer_write_n_bits_64_bits() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        bits_writer.write_n_bits(0x0102030405060708, 64).unwrap();

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(
            bytes.to_vec(),
            vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        );
    }

    // ============================================
    // write_bytes Tests
    // ============================================

    #[test]
    fn test_bits_writer_write_bytes() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        let mut data = BytesMut::new();
        data.extend_from_slice(&[0x01, 0x02, 0x03]);
        bits_writer.write_bytes(data).unwrap();

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0x01, 0x02, 0x03]);
    }

    // ============================================
    // bits_aligment_8 Tests
    // ============================================

    #[test]
    fn test_bits_writer_bits_aligment_8_original() {
        // Original test preserved
        let bytes_writer = BytesWriter::new();
        let mut bit_writer = BitsWriter::new(bytes_writer);

        bit_writer.write_bit(1).unwrap();
        bit_writer.write_bit(1).unwrap();
        bit_writer.write_bit(0).unwrap();

        bit_writer.bits_alignment_8().unwrap();

        let byte = bit_writer.get_current_bytes();
        assert!(byte.to_vec()[0] == 0xC0); //0x11000000

        bit_writer.write_bit(1).unwrap();
        bit_writer.write_bit(1).unwrap();
        bit_writer.write_bit(0).unwrap();

        assert!(bit_writer.cur_bit_num == 3);
        assert!(bit_writer.cur_byte == 0xC0); //0x11000000
    }

    #[test]
    fn test_bits_writer_bits_aligment_8_flush_to_byte() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // Write 3 bits
        bits_writer.write_n_bits(0b111, 3).unwrap();

        // Align - should pad with zeros to make a full byte
        bits_writer.bits_alignment_8().unwrap();

        let bytes = bits_writer.get_current_bytes();
        // 111 + 00000 padding = 11100000 = 0xE0
        assert_eq!(bytes.to_vec(), vec![0xE0]);
    }

    #[test]
    fn test_bits_writer_bits_aligment_8_already_aligned() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // Write full byte
        bits_writer.write_n_bits(0xFF, 8).unwrap();

        // Already aligned - should be no-op
        bits_writer.bits_alignment_8().unwrap();

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0xFF]);
    }

    // ============================================
    // len and is_empty Tests
    // ============================================

    #[test]
    fn test_bits_writer_len_increases_on_write() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        assert_eq!(bits_writer.len(), 0);

        bits_writer.write_bit(1).unwrap();
        assert_eq!(bits_writer.len(), 1);

        bits_writer.write_n_bits(0xFF, 8).unwrap();
        assert_eq!(bits_writer.len(), 9);
    }

    #[test]
    fn test_bits_writer_is_empty() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        assert!(bits_writer.is_empty());

        bits_writer.write_bit(1).unwrap();

        assert!(!bits_writer.is_empty());
    }

    // ============================================
    // Roundtrip Tests (Writer -> Reader)
    // ============================================

    #[test]
    fn test_bits_writer_reader_roundtrip_simple() {
        use super::super::bits_reader::BitsReader;
        use super::super::bytes_reader::BytesReader;

        // Write bits
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        bits_writer.write_n_bits(0b10101010, 8).unwrap();

        // Read back
        let written_bytes = bits_writer.get_current_bytes();
        let mut bytes_buf = BytesMut::new();
        bytes_buf.extend_from_slice(&written_bytes[..]);
        let bytes_reader = BytesReader::new(bytes_buf);
        let mut bits_reader = BitsReader::new(bytes_reader);

        assert_eq!(bits_reader.read_n_bits(8).unwrap(), 0b10101010);
    }

    #[test]
    fn test_bits_writer_reader_roundtrip_complex() {
        use super::super::bits_reader::BitsReader;
        use super::super::bytes_reader::BytesReader;

        // Write various bit lengths
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        bits_writer.write_n_bits(0b101, 3).unwrap(); // 3 bits
        bits_writer.write_n_bits(0b11, 2).unwrap(); // 2 bits
        bits_writer.write_n_bits(0b010, 3).unwrap(); // 3 bits
        bits_writer.write_n_bits(0xABCD, 16).unwrap(); // 16 bits
        bits_writer.bits_alignment_8().unwrap(); // Align remaining

        // Read back
        let written_bytes = bits_writer.get_current_bytes();
        let mut bytes_buf = BytesMut::new();
        bytes_buf.extend_from_slice(&written_bytes[..]);
        let bytes_reader = BytesReader::new(bytes_buf);
        let mut bits_reader = BitsReader::new(bytes_reader);

        assert_eq!(bits_reader.read_n_bits(3).unwrap(), 0b101);
        assert_eq!(bits_reader.read_n_bits(2).unwrap(), 0b11);
        assert_eq!(bits_reader.read_n_bits(3).unwrap(), 0b010);
        assert_eq!(bits_reader.read_n_bits(16).unwrap(), 0xABCD);
    }

    // ============================================
    // Complex Scenarios
    // ============================================

    #[test]
    fn test_bits_writer_h264_style_nal_header() {
        // Simulate writing H.264 NAL unit header
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // forbidden_zero_bit(1) | nal_ref_idc(2) | nal_unit_type(5)
        // 0 | 11 | 00101 = 0x65 (IDR slice)
        bits_writer.write_bit(0).unwrap(); // forbidden_zero_bit
        bits_writer.write_n_bits(3, 2).unwrap(); // nal_ref_idc = 3
        bits_writer.write_n_bits(5, 5).unwrap(); // nal_unit_type = 5

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0x65]);
    }

    #[test]
    fn test_bits_writer_rtp_header_style() {
        // Simulate writing RTP header first byte
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // version(2) | padding(1) | extension(1) | cc(4)
        // 10 | 0 | 0 | 0000 = 0x80
        bits_writer.write_n_bits(2, 2).unwrap(); // version = 2
        bits_writer.write_bit(0).unwrap(); // padding = 0
        bits_writer.write_bit(0).unwrap(); // extension = 0
        bits_writer.write_n_bits(0, 4).unwrap(); // cc = 0

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0x80]);
    }

    #[test]
    fn test_bits_writer_mixed_operations() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // Write some bits
        bits_writer.write_n_bits(0x0F, 4).unwrap();

        // Align
        bits_writer.bits_alignment_8().unwrap();

        // Write full byte
        bits_writer.write_8bit(0xAB).unwrap();

        // Write more bits
        bits_writer.write_n_bits(0xCD, 8).unwrap();

        let bytes = bits_writer.get_current_bytes();
        // 0x0F (4 bits) + padding -> 0xF0, then 0xAB, then 0xCD
        assert_eq!(bytes.to_vec(), vec![0xF0, 0xAB, 0xCD]);
    }

    #[test]
    fn test_bits_writer_many_small_writes() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // Write 16 single bits to form 0xAAAA
        for i in 0..16 {
            bits_writer
                .write_bit(if i % 2 == 0 { 1 } else { 0 })
                .unwrap();
        }

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0xAA, 0xAA]);
    }

    #[test]
    fn test_bits_writer_sequential_n_bits() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // Write sequence: 3 bits, 5 bits, 8 bits
        bits_writer.write_n_bits(0b111, 3).unwrap();
        bits_writer.write_n_bits(0b00000, 5).unwrap();
        bits_writer.write_n_bits(0xFF, 8).unwrap();

        let bytes = bits_writer.get_current_bytes();
        // 111 + 00000 = 11100000 = 0xE0, then 0xFF
        assert_eq!(bytes.to_vec(), vec![0xE0, 0xFF]);
    }

    // ============================================
    // Property-based Tests (proptest)
    // ============================================

    #[test]
    fn test_bits_writer_read_bit_property() {
        use super::super::bits_reader::BitsReader;
        use super::super::bytes_reader::BytesReader;
        use proptest::prelude::*;

        proptest!(|(byte in 0u8..=255u8)| {
            // Write byte as individual bits
            let bytes_writer = BytesWriter::new();
            let mut bits_writer = BitsWriter::new(bytes_writer);

            for i in 0..8 {
                let bit = (byte >> (7 - i)) & 0x01;
                bits_writer.write_bit(bit).unwrap();
            }

            // Read back using BitsReader
            let written_bytes = bits_writer.get_current_bytes();
            let mut bytes_buf = BytesMut::new();
            bytes_buf.extend_from_slice(&written_bytes[..]);
            let bytes_reader = BytesReader::new(bytes_buf);
            let mut bits_reader = BitsReader::new(bytes_reader);

            let mut reconstructed = 0u8;
            for _ in 0..8 {
                let bit = bits_reader.read_bit().unwrap();
                reconstructed = (reconstructed << 1) | bit;
            }

            assert_eq!(reconstructed, byte);
        });
    }

    #[test]
    fn test_bits_writer_read_n_bits_property() {
        use super::super::bits_reader::BitsReader;
        use super::super::bytes_reader::BytesReader;
        use proptest::prelude::*;

        proptest!(|(value in 0u64..=0xFFFFFFFFu64, bit_count in 1usize..=32usize)| {
            // Mask value to fit in bit_count bits
            let masked_value = value & ((1u64 << bit_count) - 1);

            // Write bits
            let bytes_writer = BytesWriter::new();
            let mut bits_writer = BitsWriter::new(bytes_writer);
            bits_writer.write_n_bits(masked_value, bit_count).unwrap();
            bits_writer.bits_alignment_8().unwrap();

            // Read back
            let written_bytes = bits_writer.get_current_bytes();
            let mut bytes_buf = BytesMut::new();
            bytes_buf.extend_from_slice(&written_bytes[..]);
            let bytes_reader = BytesReader::new(bytes_buf);
            let mut bits_reader = BitsReader::new(bytes_reader);

            let read_value = bits_reader.read_n_bits(bit_count).unwrap();
            assert_eq!(read_value, masked_value);
        });
    }

    #[test]
    fn test_bits_writer_read_8bit_property() {
        use super::super::bits_reader::BitsReader;
        use super::super::bytes_reader::BytesReader;
        use proptest::prelude::*;

        proptest!(|(byte in 0u8..=255u8)| {
            // Write byte
            let bytes_writer = BytesWriter::new();
            let mut bits_writer = BitsWriter::new(bytes_writer);
            bits_writer.write_8bit(byte).unwrap();

            // Read back
            let written_bytes = bits_writer.get_current_bytes();
            let mut bytes_buf = BytesMut::new();
            bytes_buf.extend_from_slice(&written_bytes[..]);
            let bytes_reader = BytesReader::new(bytes_buf);
            let mut bits_reader = BitsReader::new(bytes_reader);

            assert_eq!(bits_reader.read_byte().unwrap(), byte);
        });
    }

    // ============================================
    // Additional Error Condition Tests
    // ============================================

    #[test]
    fn test_bits_writer_write_n_bits_64_bits_max() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // Write maximum 64 bits
        bits_writer.write_n_bits(0xFFFFFFFFFFFFFFFF, 64).unwrap();
        bits_writer.bits_alignment_8().unwrap();

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.len(), 8);
    }

    #[test]
    fn test_bits_writer_write_8bit_after_bits_error() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // Write partial bits
        bits_writer.write_n_bits(0b111, 3).unwrap();

        // Try to write 8bit - should fail
        let result = bits_writer.write_8bit(0x42);
        assert!(result.is_err());
        match result.unwrap_err().value {
            BitErrorValue::CannotWrite8Bit => {}
            _ => panic!("Expected CannotWrite8Bit error"),
        }
    }

    #[test]
    fn test_bits_writer_write_bytes_after_bits() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // Write some bits
        bits_writer.write_n_bits(0b111, 3).unwrap();

        // Write bytes requires byte alignment, so partial bits are flushed first
        let mut data = BytesMut::new();
        data.extend_from_slice(&[0xAB, 0xCD]);
        bits_writer.write_bytes(data).unwrap();

        let bytes = bits_writer.get_current_bytes();
        // Partial bits flushed (1 byte) + 2 bytes written = 3 bytes total
        assert_eq!(bytes.len(), 3);
        assert_eq!(bits_writer.cur_bit_num, 0); // no partial bits remain after flush
        // First byte has 3 bits in MSB positions: 0b111xxxxx = 0xE0
        assert_eq!(bytes[0], 0xE0);
        assert_eq!(bytes[1], 0xAB);
        assert_eq!(bytes[2], 0xCD);
    }

    #[test]
    fn test_bits_writer_bits_alignment_8_no_partial_bits() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // Write full byte
        bits_writer.write_n_bits(0xFF, 8).unwrap();

        // Align (should be no-op)
        bits_writer.bits_alignment_8().unwrap();

        let bytes = bits_writer.get_current_bytes();
        assert_eq!(bytes.to_vec(), vec![0xFF]);
    }

    #[test]
    fn test_bits_writer_len_accuracy() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        assert_eq!(bits_writer.len(), 0);

        bits_writer.write_bit(1).unwrap();
        assert_eq!(bits_writer.len(), 1);

        bits_writer.write_n_bits(0xFF, 8).unwrap();
        // 1 bit + 8 bits = 9 bits, first byte flushed, 1 partial bit remaining
        assert_eq!(bits_writer.len(), 9);

        bits_writer.bits_alignment_8().unwrap();
        // After alignment, partial byte (1 bit + 7 zero padding) is flushed
        // Total: 2 bytes = 16 bits
        assert_eq!(bits_writer.len(), 16);
    }

    #[test]
    fn test_bits_writer_is_empty_after_clear() {
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        bits_writer.write_n_bits(0xFF, 8).unwrap();
        assert!(!bits_writer.is_empty());

        // Clear underlying writer
        bits_writer.writer.clear();
        bits_writer.bits_alignment_8().unwrap();

        // Should be empty if no partial bits
        assert!(bits_writer.is_empty());
    }
}
