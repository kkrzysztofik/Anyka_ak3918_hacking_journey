use {
    super::bits_errors::{BitError, BitErrorValue},
    super::bytes_reader::BytesReader,
    bytes::BytesMut,
};

pub struct BitsReader {
    reader: BytesReader,
    cur_byte: u8,
    cur_bit_left: u8,
}

impl BitsReader {
    pub fn new(reader: BytesReader) -> Self {
        Self {
            reader,
            cur_byte: 0,
            cur_bit_left: 0,
        }
    }

    pub fn extend_data(&mut self, bytes: BytesMut) {
        self.reader.extend_from_slice(&bytes[..]);
    }

    pub fn len(&self) -> usize {
        self.reader.len() * 8 + self.cur_bit_left as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn read_byte(&mut self) -> Result<u8, BitError> {
        if self.cur_bit_left != 0 {
            return Err(BitError {
                value: BitErrorValue::CannotReadByte,
            });
        }

        let byte = self.reader.read_u8()?;
        Ok(byte)
    }

    pub fn read_bit(&mut self) -> Result<u8, BitError> {
        if self.cur_bit_left == 0 {
            self.cur_byte = self.reader.read_u8()?;
            self.cur_bit_left = 8;
        }
        self.cur_bit_left -= 1;
        Ok((self.cur_byte >> self.cur_bit_left) & 0x01)
    }

    pub fn read_n_bits(&mut self, n: usize) -> Result<u64, BitError> {
        let mut result: u64 = 0;
        for _ in 0..n {
            result <<= 1;
            let cur_bit = self.read_bit()?;
            result |= cur_bit as u64;
        }
        Ok(result)
    }

    pub fn bits_aligment_8(&mut self) {
        self.cur_bit_left = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    // ============================================
    // BitsReader Construction and Basic Operations
    // ============================================

    #[test]
    fn test_bits_reader_new_empty() {
        let bytes_reader = BytesReader::new(BytesMut::new());
        let bits_reader = BitsReader::new(bytes_reader);
        assert_eq!(bits_reader.len(), 0);
        assert!(bits_reader.is_empty());
    }

    #[test]
    fn test_bits_reader_new_with_data() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xFF, 0x00]);
        let bits_reader = BitsReader::new(bytes_reader);
        assert_eq!(bits_reader.len(), 16); // 2 bytes = 16 bits
        assert!(!bits_reader.is_empty());
    }

    #[test]
    fn test_bits_reader_extend_data() {
        let bytes_reader = BytesReader::new(BytesMut::new());
        let mut bits_reader = BitsReader::new(bytes_reader);

        let mut additional = BytesMut::new();
        additional.extend_from_slice(&[0xFF, 0x00, 0xAB]);
        bits_reader.extend_data(additional);

        assert_eq!(bits_reader.len(), 24); // 3 bytes = 24 bits
    }

    // ============================================
    // read_bit Tests
    // ============================================

    #[test]
    fn test_bits_reader_read_bit_all_zeros() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0x00]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        for _ in 0..8 {
            assert_eq!(bits_reader.read_bit().unwrap(), 0);
        }
    }

    #[test]
    fn test_bits_reader_read_bit_all_ones() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xFF]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        for _ in 0..8 {
            assert_eq!(bits_reader.read_bit().unwrap(), 1);
        }
    }

    #[test]
    fn test_bits_reader_read_bit_alternating() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        // 0xAA = 10101010
        bytes_reader.extend_from_slice(&[0xAA]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        assert_eq!(bits_reader.read_bit().unwrap(), 1);
        assert_eq!(bits_reader.read_bit().unwrap(), 0);
        assert_eq!(bits_reader.read_bit().unwrap(), 1);
        assert_eq!(bits_reader.read_bit().unwrap(), 0);
        assert_eq!(bits_reader.read_bit().unwrap(), 1);
        assert_eq!(bits_reader.read_bit().unwrap(), 0);
        assert_eq!(bits_reader.read_bit().unwrap(), 1);
        assert_eq!(bits_reader.read_bit().unwrap(), 0);
    }

    #[test]
    fn test_bits_reader_read_bit_original() {
        // Original test preserved
        let mut bytes_reader = BytesReader::new(BytesMut::new());

        let data_0 = 2u8;
        bytes_reader.extend_from_slice(&[data_0]);
        let data_1 = 7u8;
        bytes_reader.extend_from_slice(&[data_1]);

        let mut bit_reader = BitsReader::new(bytes_reader);

        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 0);

        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 1);
        assert!(bit_reader.read_bit().unwrap() == 0);

        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 0);

        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 1);
        assert!(bit_reader.read_bit().unwrap() == 1);
        assert!(bit_reader.read_bit().unwrap() == 1);
    }

    #[test]
    fn test_bits_reader_read_bit_high_nibble() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        // 0xF0 = 11110000
        bytes_reader.extend_from_slice(&[0xF0]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        // High nibble
        assert_eq!(bits_reader.read_bit().unwrap(), 1);
        assert_eq!(bits_reader.read_bit().unwrap(), 1);
        assert_eq!(bits_reader.read_bit().unwrap(), 1);
        assert_eq!(bits_reader.read_bit().unwrap(), 1);
        // Low nibble
        assert_eq!(bits_reader.read_bit().unwrap(), 0);
        assert_eq!(bits_reader.read_bit().unwrap(), 0);
        assert_eq!(bits_reader.read_bit().unwrap(), 0);
        assert_eq!(bits_reader.read_bit().unwrap(), 0);
    }

    #[test]
    fn test_bits_reader_read_bit_not_enough_data() {
        let bytes_reader = BytesReader::new(BytesMut::new());
        let mut bits_reader = BitsReader::new(bytes_reader);

        let result = bits_reader.read_bit();
        assert!(result.is_err());
    }

    // ============================================
    // read_byte Tests
    // ============================================

    #[test]
    fn test_bits_reader_read_byte_success() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0x42, 0x43]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        assert_eq!(bits_reader.read_byte().unwrap(), 0x42);
        assert_eq!(bits_reader.read_byte().unwrap(), 0x43);
    }

    #[test]
    fn test_bits_reader_read_byte_boundary_values() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0x00, 0x7F, 0x80, 0xFF]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        assert_eq!(bits_reader.read_byte().unwrap(), 0x00);
        assert_eq!(bits_reader.read_byte().unwrap(), 0x7F);
        assert_eq!(bits_reader.read_byte().unwrap(), 0x80);
        assert_eq!(bits_reader.read_byte().unwrap(), 0xFF);
    }

    #[test]
    fn test_bits_reader_read_byte_after_bits_error() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xFF, 0x00]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        // Read some bits first
        bits_reader.read_bit().unwrap();

        // Now try to read a byte - should fail because we're not aligned
        let result = bits_reader.read_byte();
        assert!(result.is_err());
        match result.unwrap_err().value {
            BitErrorValue::CannotReadByte => {}
            _ => panic!("Expected CannotReadByte error"),
        }
    }

    // ============================================
    // read_n_bits Tests
    // ============================================

    #[test]
    fn test_bits_reader_read_n_bits_1_bit() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0x80]); // 10000000
        let mut bits_reader = BitsReader::new(bytes_reader);

        assert_eq!(bits_reader.read_n_bits(1).unwrap(), 1);
    }

    #[test]
    fn test_bits_reader_read_n_bits_4_bits() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xF0]); // 11110000
        let mut bits_reader = BitsReader::new(bytes_reader);

        assert_eq!(bits_reader.read_n_bits(4).unwrap(), 0x0F);
    }

    #[test]
    fn test_bits_reader_read_n_bits_8_bits() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xAB]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        assert_eq!(bits_reader.read_n_bits(8).unwrap(), 0xAB);
    }

    #[test]
    fn test_bits_reader_read_n_bits_16_bits() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0x12, 0x34]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        assert_eq!(bits_reader.read_n_bits(16).unwrap(), 0x1234);
    }

    #[test]
    fn test_bits_reader_read_n_bits_24_bits() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0x12, 0x34, 0x56]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        assert_eq!(bits_reader.read_n_bits(24).unwrap(), 0x123456);
    }

    #[test]
    fn test_bits_reader_read_n_bits_32_bits() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0x12, 0x34, 0x56, 0x78]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        assert_eq!(bits_reader.read_n_bits(32).unwrap(), 0x12345678);
    }

    #[test]
    fn test_bits_reader_read_n_bits_cross_byte_boundary() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xFF, 0x00]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        // Read 4 bits (1111), then read 8 bits (1111 0000) crossing the byte boundary
        assert_eq!(bits_reader.read_n_bits(4).unwrap(), 0x0F);
        assert_eq!(bits_reader.read_n_bits(8).unwrap(), 0xF0);
    }

    #[test]
    fn test_bits_reader_read_n_bits_original() {
        // Original test preserved
        let mut bytes_reader = BytesReader::new(BytesMut::new());

        let data_0 = 2u8;
        bytes_reader.extend_from_slice(&[data_0]);
        let data_1 = 7u8;
        bytes_reader.extend_from_slice(&[data_1]);
        bytes_reader.extend_from_slice(&[0b00000010]);

        let mut bit_reader = BitsReader::new(bytes_reader);
        assert!(bit_reader.read_n_bits(16).unwrap() == 0x207);

        assert!(bit_reader.read_n_bits(5).unwrap() == 0);

        assert!(bit_reader.read_n_bits(3).unwrap() == 2);
    }

    #[test]
    fn test_bits_reader_read_n_bits_zero() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xFF]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        // Reading 0 bits should return 0
        assert_eq!(bits_reader.read_n_bits(0).unwrap(), 0);
        // Buffer should be unchanged
        assert_eq!(bits_reader.len(), 8);
    }

    #[test]
    fn test_bits_reader_read_n_bits_sequential_small() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        // 0b11100101 = 0xE5
        bytes_reader.extend_from_slice(&[0xE5]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        assert_eq!(bits_reader.read_n_bits(3).unwrap(), 0b111); // 7
        assert_eq!(bits_reader.read_n_bits(2).unwrap(), 0b00);  // 0
        assert_eq!(bits_reader.read_n_bits(3).unwrap(), 0b101); // 5
    }

    // ============================================
    // bits_aligment_8 Tests
    // ============================================

    #[test]
    fn test_bits_reader_bits_aligment_8_original() {
        // Original test preserved
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        let data_0 = 2u8;
        bytes_reader.extend_from_slice(&[data_0]);
        let data_1 = 7u8;
        bytes_reader.extend_from_slice(&[data_1]);

        let mut bit_reader = BitsReader::new(bytes_reader);

        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 0);

        bit_reader.bits_aligment_8();

        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 0);

        assert!(bit_reader.read_bit().unwrap() == 0);
        assert!(bit_reader.read_bit().unwrap() == 1);
        assert!(bit_reader.read_bit().unwrap() == 1);
        assert!(bit_reader.read_bit().unwrap() == 1);
    }

    #[test]
    fn test_bits_reader_bits_aligment_8_no_op_when_aligned() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xFF, 0x00]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        // Read full byte
        bits_reader.read_n_bits(8).unwrap();

        // Already aligned, should be a no-op
        bits_reader.bits_aligment_8();

        // Should still read from second byte
        assert_eq!(bits_reader.read_n_bits(8).unwrap(), 0x00);
    }

    #[test]
    fn test_bits_reader_bits_aligment_8_skip_partial() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xFF, 0xAB]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        // Read 3 bits
        bits_reader.read_n_bits(3).unwrap();

        // Align - should skip remaining 5 bits of first byte
        bits_reader.bits_aligment_8();

        // Now should read from second byte
        assert_eq!(bits_reader.read_byte().unwrap(), 0xAB);
    }

    // ============================================
    // len and is_empty Tests
    // ============================================

    #[test]
    fn test_bits_reader_len_decreases_on_read() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xFF, 0x00]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        assert_eq!(bits_reader.len(), 16);

        bits_reader.read_bit().unwrap();
        // After reading 1 bit: 7 bits left in current byte + 8 bits from next byte = 15
        assert_eq!(bits_reader.len(), 15);

        bits_reader.read_n_bits(7).unwrap();
        // After reading 7 more bits: first byte consumed, 8 bits remaining
        assert_eq!(bits_reader.len(), 8);
    }

    #[test]
    fn test_bits_reader_is_empty_after_full_read() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xFF]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        assert!(!bits_reader.is_empty());

        bits_reader.read_n_bits(8).unwrap();

        assert!(bits_reader.is_empty());
    }

    // ============================================
    // Complex Scenarios
    // ============================================

    #[test]
    fn test_bits_reader_h264_style_parsing() {
        // Simulates reading H.264 NAL unit header bits
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        // NAL header: forbidden_zero_bit(1) | nal_ref_idc(2) | nal_unit_type(5)
        // 0x65 = 0b01100101 = 0 | 11 | 00101 (IDR slice)
        bytes_reader.extend_from_slice(&[0x65]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        let forbidden_zero_bit = bits_reader.read_bit().unwrap();
        let nal_ref_idc = bits_reader.read_n_bits(2).unwrap();
        let nal_unit_type = bits_reader.read_n_bits(5).unwrap();

        assert_eq!(forbidden_zero_bit, 0);
        assert_eq!(nal_ref_idc, 3); // 11 binary = 3
        assert_eq!(nal_unit_type, 5); // 00101 binary = 5 (IDR slice)
    }

    #[test]
    fn test_bits_reader_rtp_header_style_parsing() {
        // Simulates reading RTP header bits
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        // RTP Header first byte: version(2) | padding(1) | extension(1) | cc(4)
        // 0x80 = 0b10000000 = 10 | 0 | 0 | 0000
        bytes_reader.extend_from_slice(&[0x80]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        let version = bits_reader.read_n_bits(2).unwrap();
        let padding = bits_reader.read_bit().unwrap();
        let extension = bits_reader.read_bit().unwrap();
        let cc = bits_reader.read_n_bits(4).unwrap();

        assert_eq!(version, 2);
        assert_eq!(padding, 0);
        assert_eq!(extension, 0);
        assert_eq!(cc, 0);
    }

    #[test]
    fn test_bits_reader_mixed_operations() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xAB, 0xCD, 0xEF]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        // Read 4 bits
        assert_eq!(bits_reader.read_n_bits(4).unwrap(), 0x0A);

        // Align to byte boundary
        bits_reader.bits_aligment_8();

        // Read full byte
        assert_eq!(bits_reader.read_byte().unwrap(), 0xCD);

        // Read remaining byte as bits
        assert_eq!(bits_reader.read_n_bits(8).unwrap(), 0xEF);

        assert!(bits_reader.is_empty());
    }

    // ============================================
    // Property-based Tests (proptest)
    // ============================================

    #[test]
    fn test_bits_reader_read_bit_property() {
        use proptest::prelude::*;
        proptest!(|(byte in 0u8..=255u8)| {
            let mut bytes_reader = BytesReader::new(BytesMut::new());
            bytes_reader.extend_from_slice(&[byte]);
            let mut bits_reader = BitsReader::new(bytes_reader);

            // Read all 8 bits
            let mut bits = Vec::new();
            for _ in 0..8 {
                bits.push(bits_reader.read_bit().unwrap());
            }

            // Verify we can reconstruct the byte
            let mut reconstructed = 0u8;
            for &bit in &bits {
                reconstructed = (reconstructed << 1) | bit;
            }
            assert_eq!(reconstructed, byte);
        });
    }

    #[test]
    fn test_bits_reader_read_n_bits_property() {
        use proptest::prelude::*;
        proptest!(|(byte in 0u8..=255u8, bit_count in 1usize..=8usize)| {
            let mut bytes_reader = BytesReader::new(BytesMut::new());
            bytes_reader.extend_from_slice(&[byte]);
            let mut bits_reader = BitsReader::new(bytes_reader);

            let result = bits_reader.read_n_bits(bit_count).unwrap();
            // Result should be in valid range for bit_count bits
            assert!(result < (1u64 << bit_count));
        });
    }

    #[test]
    fn test_bits_reader_read_byte_property() {
        use proptest::prelude::*;
        proptest!(|(byte in 0u8..=255u8)| {
            let mut bytes_reader = BytesReader::new(BytesMut::new());
            bytes_reader.extend_from_slice(&[byte]);
            let mut bits_reader = BitsReader::new(bytes_reader);

            assert_eq!(bits_reader.read_byte().unwrap(), byte);
        });
    }

    // ============================================
    // Additional Error Condition Tests
    // ============================================

    #[test]
    fn test_bits_reader_read_bit_empty_buffer() {
        let bytes_reader = BytesReader::new(BytesMut::new());
        let mut bits_reader = BitsReader::new(bytes_reader);

        let result = bits_reader.read_bit();
        assert!(result.is_err());
    }

    #[test]
    fn test_bits_reader_read_n_bits_not_enough_data() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xFF]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        // Try to read more than 8 bits
        let result = bits_reader.read_n_bits(16);
        assert!(result.is_err());
    }

    #[test]
    fn test_bits_reader_read_byte_empty_buffer() {
        let bytes_reader = BytesReader::new(BytesMut::new());
        let mut bits_reader = BitsReader::new(bytes_reader);

        let result = bits_reader.read_byte();
        assert!(result.is_err());
    }

    #[test]
    fn test_bits_reader_extend_data_after_read() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xFF]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        // Read some bits
        bits_reader.read_n_bits(4).unwrap();

        // Extend with more data
        let mut additional = BytesMut::new();
        additional.extend_from_slice(&[0x00, 0xAB]);
        bits_reader.extend_data(additional);

        // Should be able to read remaining bits from first byte + new bytes
        assert_eq!(bits_reader.read_n_bits(4).unwrap(), 0x0F);
        assert_eq!(bits_reader.read_byte().unwrap(), 0x00);
        assert_eq!(bits_reader.read_byte().unwrap(), 0xAB);
    }

    #[test]
    fn test_bits_reader_len_accuracy() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xFF, 0x00, 0xAB]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        assert_eq!(bits_reader.len(), 24); // 3 bytes = 24 bits

        bits_reader.read_n_bits(5).unwrap();
        assert_eq!(bits_reader.len(), 19); // 24 - 5 = 19

        bits_reader.read_n_bits(3).unwrap();
        assert_eq!(bits_reader.len(), 16); // 19 - 3 = 16 (one byte consumed)
    }

    #[test]
    fn test_bits_reader_bits_alignment_8_multiple_times() {
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0xFF, 0x00, 0xAB]);
        let mut bits_reader = BitsReader::new(bytes_reader);

        bits_reader.read_n_bits(3).unwrap();
        bits_reader.bits_aligment_8();
        bits_reader.bits_aligment_8(); // Should be no-op
        bits_reader.bits_aligment_8(); // Should be no-op

        // Should read from second byte
        assert_eq!(bits_reader.read_byte().unwrap(), 0x00);
    }
}
