use {
    super::{
        TNetIO,
        bytes_errors::{BytesReadError, BytesReadErrorValue},
    },
    byteorder::{ByteOrder, ReadBytesExt},
    bytes::{BufMut, BytesMut},
    std::{io::Cursor, sync::Arc},
    tokio::sync::Mutex,
};

pub struct BytesReader {
    buffer: BytesMut,
}
impl BytesReader {
    pub fn new(input: BytesMut) -> Self {
        Self { buffer: input }
    }

    pub fn extend_from_slice(&mut self, extend: &[u8]) {
        let remaining_mut = self.buffer.remaining_mut();
        let extend_length = extend.len();

        if extend_length > remaining_mut {
            let additional = extend_length - remaining_mut;
            self.buffer.reserve(additional);
        }

        self.buffer.extend_from_slice(extend)
    }

    pub fn read_bytes(&mut self, bytes_num: usize) -> Result<BytesMut, BytesReadError> {
        if self.buffer.len() < bytes_num {
            return Err(BytesReadError {
                value: BytesReadErrorValue::NotEnoughBytes,
            });
        }
        Ok(self.buffer.split_to(bytes_num))
    }

    pub fn advance_bytes(&mut self, bytes_num: usize) -> Result<BytesMut, BytesReadError> {
        if self.buffer.len() < bytes_num {
            return Err(BytesReadError {
                value: BytesReadErrorValue::NotEnoughBytes,
            });
        }

        // Use split_to directly without cloning to avoid unnecessary memory copy
        Ok(self.buffer.split_to(bytes_num))
    }

    pub fn read_bytes_cursor(
        &mut self,
        bytes_num: usize,
    ) -> Result<Cursor<BytesMut>, BytesReadError> {
        let tmp_bytes = self.read_bytes(bytes_num)?;
        let tmp_cursor = Cursor::new(tmp_bytes);
        Ok(tmp_cursor)
    }

    pub fn advance_bytes_cursor(
        &mut self,
        bytes_num: usize,
    ) -> Result<Cursor<BytesMut>, BytesReadError> {
        let tmp_bytes = self.advance_bytes(bytes_num)?;
        let tmp_cursor = Cursor::new(tmp_bytes);
        Ok(tmp_cursor)
    }

    pub fn read_u8(&mut self) -> Result<u8, BytesReadError> {
        let mut cursor = self.read_bytes_cursor(1)?;

        Ok(cursor.read_u8()?)
    }

    pub fn advance_u8(&mut self) -> Result<u8, BytesReadError> {
        let mut cursor = self.advance_bytes_cursor(1)?;
        Ok(cursor.read_u8()?)
    }

    pub fn read_u16<T: ByteOrder>(&mut self) -> Result<u16, BytesReadError> {
        let mut cursor = self.read_bytes_cursor(2)?;
        let val = cursor.read_u16::<T>()?;
        Ok(val)
    }

    pub fn read_u24<T: ByteOrder>(&mut self) -> Result<u32, BytesReadError> {
        let mut cursor = self.read_bytes_cursor(3)?;
        let val = cursor.read_u24::<T>()?;
        Ok(val)
    }

    pub fn advance_u24<T: ByteOrder>(&mut self) -> Result<u32, BytesReadError> {
        let mut cursor = self.advance_bytes_cursor(3)?;
        Ok(cursor.read_u24::<T>()?)
    }

    pub fn read_u32<T: ByteOrder>(&mut self) -> Result<u32, BytesReadError> {
        let mut cursor = self.read_bytes_cursor(4)?;
        let val = cursor.read_u32::<T>()?;

        Ok(val)
    }

    pub fn read_u48<T: ByteOrder>(&mut self) -> Result<u64, BytesReadError> {
        let mut cursor = self.read_bytes_cursor(6)?;
        let val = cursor.read_u48::<T>()?;

        Ok(val)
    }

    pub fn read_f64<T: ByteOrder>(&mut self) -> Result<f64, BytesReadError> {
        let mut cursor = self.read_bytes_cursor(8)?;
        let val = cursor.read_f64::<T>()?;

        Ok(val)
    }

    pub fn read_u64<T: ByteOrder>(&mut self) -> Result<u64, BytesReadError> {
        let mut cursor = self.read_bytes_cursor(8)?;
        let val = cursor.read_u64::<T>()?;

        Ok(val)
    }

    pub fn get(&self, index: usize) -> Result<u8, BytesReadError> {
        if index >= self.len() {
            return Err(BytesReadError {
                value: BytesReadErrorValue::IndexOutofRange,
            });
        }

        self.buffer.get(index).copied().ok_or(BytesReadError {
            value: BytesReadErrorValue::IndexOutofRange,
        })
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn extract_remaining_bytes(&mut self) -> BytesMut {
        self.buffer.split_to(self.buffer.len())
    }
    pub fn get_remaining_bytes(&self) -> BytesMut {
        self.buffer.clone()
    }
}
pub struct AsyncBytesReader<T1: TNetIO> {
    pub bytes_reader: BytesReader,
    pub io: Arc<Mutex<T1>>,
}

impl<T1> AsyncBytesReader<T1>
where
    T1: TNetIO,
{
    pub fn new(io: Arc<Mutex<T1>>) -> Self {
        Self {
            bytes_reader: BytesReader::new(BytesMut::default()),
            io,
        }
    }

    pub async fn read(&mut self) -> Result<(), BytesReadError> {
        let data = self.io.lock().await.read().await?;
        self.bytes_reader.extend_from_slice(&data[..]);
        Ok(())
    }

    async fn check(&mut self, bytes_num: usize) -> Result<(), BytesReadError> {
        while self.bytes_reader.len() < bytes_num {
            self.read().await?;
        }

        Ok(())
    }

    pub async fn read_bytes(&mut self, bytes_num: usize) -> Result<BytesMut, BytesReadError> {
        self.check(bytes_num).await?;
        self.bytes_reader.read_bytes(bytes_num)
    }

    pub async fn advance_bytes(&mut self, bytes_num: usize) -> Result<BytesMut, BytesReadError> {
        self.check(bytes_num).await?;
        self.bytes_reader.advance_bytes(bytes_num)
    }

    pub async fn read_bytes_cursor(
        &mut self,
        bytes_num: usize,
    ) -> Result<Cursor<BytesMut>, BytesReadError> {
        self.check(bytes_num).await?;
        self.bytes_reader.read_bytes_cursor(bytes_num)
    }

    pub async fn advance_bytes_cursor(
        &mut self,
        bytes_num: usize,
    ) -> Result<Cursor<BytesMut>, BytesReadError> {
        self.check(bytes_num).await?;
        self.bytes_reader.advance_bytes_cursor(bytes_num)
    }

    pub async fn read_u8(&mut self) -> Result<u8, BytesReadError> {
        self.check(1).await?;
        self.bytes_reader.read_u8()
    }

    pub async fn advance_u8(&mut self) -> Result<u8, BytesReadError> {
        self.check(1).await?;
        self.bytes_reader.advance_u8()
    }

    pub async fn read_u16<T: ByteOrder>(&mut self) -> Result<u16, BytesReadError> {
        self.check(2).await?;
        self.bytes_reader.read_u16::<T>()
    }

    pub async fn read_u24<T: ByteOrder>(&mut self) -> Result<u32, BytesReadError> {
        self.check(3).await?;
        self.bytes_reader.read_u24::<T>()
    }

    pub async fn advance_u24<T: ByteOrder>(&mut self) -> Result<u32, BytesReadError> {
        self.check(3).await?;
        self.bytes_reader.advance_u24::<T>()
    }

    pub async fn read_u32<T: ByteOrder>(&mut self) -> Result<u32, BytesReadError> {
        self.check(4).await?;
        self.bytes_reader.read_u32::<T>()
    }

    pub async fn read_f64<T: ByteOrder>(&mut self) -> Result<f64, BytesReadError> {
        self.check(8).await?;
        self.bytes_reader.read_f64::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{BigEndian, LittleEndian};
    use bytes::BytesMut;
    use std::cell::RefCell;
    use std::rc::Rc;

    // ============================================
    // BytesReader Construction and Basic Operations
    // ============================================

    #[test]
    fn test_bytes_reader_new_empty() {
        let reader = BytesReader::new(BytesMut::new());
        assert_eq!(reader.len(), 0);
        assert!(reader.is_empty());
    }

    #[test]
    fn test_bytes_reader_new_with_data() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2, 3, 4, 5]);
        let reader = BytesReader::new(buf);
        assert_eq!(reader.len(), 5);
        assert!(!reader.is_empty());
    }

    #[test]
    fn test_bytes_reader_extend_from_slice() {
        let mut reader = BytesReader::new(BytesMut::new());
        reader.extend_from_slice(&[1, 2, 3]);
        assert_eq!(reader.len(), 3);

        reader.extend_from_slice(&[4, 5]);
        assert_eq!(reader.len(), 5);
    }

    #[test]
    fn test_bytes_reader_extend_from_slice_with_reserve() {
        let mut reader = BytesReader::new(BytesMut::with_capacity(2));
        // Extend beyond initial capacity to test reserve logic
        reader.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(reader.len(), 10);
    }

    // ============================================
    // read_u8 Tests
    // ============================================

    #[test]
    fn test_bytes_reader_read_u8_success() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x00, 0x7F, 0x80, 0xFF]);
        let mut reader = BytesReader::new(buf);

        assert_eq!(reader.read_u8().unwrap(), 0x00);
        assert_eq!(reader.read_u8().unwrap(), 0x7F);
        assert_eq!(reader.read_u8().unwrap(), 0x80);
        assert_eq!(reader.read_u8().unwrap(), 0xFF);
    }

    #[test]
    fn test_bytes_reader_read_u8_not_enough_bytes() {
        let mut reader = BytesReader::new(BytesMut::new());
        let result = reader.read_u8();
        assert!(result.is_err());
        match result.unwrap_err().value {
            BytesReadErrorValue::NotEnoughBytes => {}
            _ => panic!("Expected NotEnoughBytes error"),
        }
    }

    #[test]
    fn test_bytes_reader_advance_u8() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x42, 0x43]);
        let mut reader = BytesReader::new(buf);

        // advance_u8 should read without consuming
        let val = reader.advance_u8().unwrap();
        assert_eq!(val, 0x42);
        // Length should still be 2 since advance doesn't consume
        assert_eq!(reader.len(), 2);
    }

    // ============================================
    // read_u16 Tests (BigEndian and LittleEndian)
    // ============================================

    #[test]
    fn test_bytes_reader_read_u16_big_endian() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x01, 0x02]); // 0x0102 = 258
        let mut reader = BytesReader::new(buf);

        assert_eq!(reader.read_u16::<BigEndian>().unwrap(), 0x0102);
    }

    #[test]
    fn test_bytes_reader_read_u16_little_endian() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x02, 0x01]); // 0x0102 = 258 in little endian
        let mut reader = BytesReader::new(buf);

        assert_eq!(reader.read_u16::<LittleEndian>().unwrap(), 0x0102);
    }

    #[test]
    fn test_bytes_reader_read_u16_boundary_values() {
        // Min value (0)
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x00, 0x00]);
        let mut reader = BytesReader::new(buf);
        assert_eq!(reader.read_u16::<BigEndian>().unwrap(), 0);

        // Max value (65535)
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0xFF, 0xFF]);
        let mut reader = BytesReader::new(buf);
        assert_eq!(reader.read_u16::<BigEndian>().unwrap(), 65535);
    }

    #[test]
    fn test_bytes_reader_read_u16_not_enough_bytes() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x01]); // Only 1 byte
        let mut reader = BytesReader::new(buf);

        let result = reader.read_u16::<BigEndian>();
        assert!(result.is_err());
    }

    // ============================================
    // read_u24 Tests
    // ============================================

    #[test]
    fn test_bytes_reader_read_u24_big_endian() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x01, 0x02, 0x03]); // 0x010203 = 66051
        let mut reader = BytesReader::new(buf);

        assert_eq!(reader.read_u24::<BigEndian>().unwrap(), 0x010203);
    }

    #[test]
    fn test_bytes_reader_read_u24_little_endian() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x03, 0x02, 0x01]); // 0x010203 in little endian
        let mut reader = BytesReader::new(buf);

        assert_eq!(reader.read_u24::<LittleEndian>().unwrap(), 0x010203);
    }

    #[test]
    fn test_bytes_reader_read_u24_max_value() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0xFF, 0xFF, 0xFF]); // Max 24-bit value
        let mut reader = BytesReader::new(buf);

        assert_eq!(reader.read_u24::<BigEndian>().unwrap(), 0xFFFFFF);
    }

    #[test]
    fn test_bytes_reader_advance_u24() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x01, 0x02, 0x03]);
        let mut reader = BytesReader::new(buf);

        let val = reader.advance_u24::<BigEndian>().unwrap();
        assert_eq!(val, 0x010203);
        assert_eq!(reader.len(), 3); // advance doesn't consume
    }

    // ============================================
    // read_u32 Tests
    // ============================================

    #[test]
    fn test_bytes_reader_read_u32_big_endian() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]); // 0x01020304
        let mut reader = BytesReader::new(buf);

        assert_eq!(reader.read_u32::<BigEndian>().unwrap(), 0x01020304);
    }

    #[test]
    fn test_bytes_reader_read_u32_little_endian() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x04, 0x03, 0x02, 0x01]);
        let mut reader = BytesReader::new(buf);

        assert_eq!(reader.read_u32::<LittleEndian>().unwrap(), 0x01020304);
    }

    #[test]
    fn test_bytes_reader_read_u32_boundary_values() {
        // Min
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        let mut reader = BytesReader::new(buf);
        assert_eq!(reader.read_u32::<BigEndian>().unwrap(), 0);

        // Max
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
        let mut reader = BytesReader::new(buf);
        assert_eq!(reader.read_u32::<BigEndian>().unwrap(), u32::MAX);
    }

    // ============================================
    // read_u48 Tests
    // ============================================

    #[test]
    fn test_bytes_reader_read_u48_big_endian() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let mut reader = BytesReader::new(buf);

        assert_eq!(reader.read_u48::<BigEndian>().unwrap(), 0x010203040506);
    }

    #[test]
    fn test_bytes_reader_read_u48_little_endian() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x06, 0x05, 0x04, 0x03, 0x02, 0x01]);
        let mut reader = BytesReader::new(buf);

        assert_eq!(reader.read_u48::<LittleEndian>().unwrap(), 0x010203040506);
    }

    // ============================================
    // read_u64 Tests
    // ============================================

    #[test]
    fn test_bytes_reader_read_u64_big_endian() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        let mut reader = BytesReader::new(buf);

        assert_eq!(reader.read_u64::<BigEndian>().unwrap(), 0x0102030405060708);
    }

    #[test]
    fn test_bytes_reader_read_u64_little_endian() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]);
        let mut reader = BytesReader::new(buf);

        assert_eq!(
            reader.read_u64::<LittleEndian>().unwrap(),
            0x0102030405060708
        );
    }

    #[test]
    fn test_bytes_reader_read_u64_boundary_values() {
        // Max
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        let mut reader = BytesReader::new(buf);
        assert_eq!(reader.read_u64::<BigEndian>().unwrap(), u64::MAX);
    }

    // ============================================
    // read_f64 Tests
    // ============================================

    #[test]
    fn test_bytes_reader_read_f64_big_endian() {
        let mut buf = BytesMut::new();
        // IEEE 754 representation of 1.0 in big endian
        buf.extend_from_slice(&[0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        let mut reader = BytesReader::new(buf);

        let val = reader.read_f64::<BigEndian>().unwrap();
        assert!((val - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_bytes_reader_read_f64_pi() {
        let mut buf = BytesMut::new();
        // Write PI using byteorder
        let pi: f64 = std::f64::consts::PI;
        let mut bytes = [0u8; 8];
        byteorder::BigEndian::write_f64(&mut bytes, pi);
        buf.extend_from_slice(&bytes);

        let mut reader = BytesReader::new(buf);
        let val = reader.read_f64::<BigEndian>().unwrap();
        assert!((val - pi).abs() < f64::EPSILON);
    }

    // ============================================
    // read_bytes Tests
    // ============================================

    #[test]
    fn test_bytes_reader_read_bytes_success() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2, 3, 4, 5]);
        let mut reader = BytesReader::new(buf);

        let result = reader.read_bytes(3).unwrap();
        assert_eq!(&result[..], &[1, 2, 3]);
        assert_eq!(reader.len(), 2); // 2 bytes remaining
    }

    #[test]
    fn test_bytes_reader_read_bytes_exact_length() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2, 3]);
        let mut reader = BytesReader::new(buf);

        let result = reader.read_bytes(3).unwrap();
        assert_eq!(&result[..], &[1, 2, 3]);
        assert!(reader.is_empty());
    }

    #[test]
    fn test_bytes_reader_read_bytes_zero() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2, 3]);
        let mut reader = BytesReader::new(buf);

        let result = reader.read_bytes(0).unwrap();
        assert!(result.is_empty());
        assert_eq!(reader.len(), 3);
    }

    #[test]
    fn test_bytes_reader_read_bytes_not_enough() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2]);
        let mut reader = BytesReader::new(buf);

        let result = reader.read_bytes(5);
        assert!(result.is_err());
    }

    // ============================================
    // advance_bytes Tests
    // ============================================

    #[test]
    fn test_bytes_reader_advance_bytes_success() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2, 3, 4, 5]);
        let mut reader = BytesReader::new(buf);

        let result = reader.advance_bytes(3).unwrap();
        assert_eq!(&result[..], &[1, 2, 3]);
        // advance doesn't consume the buffer
        assert_eq!(reader.len(), 5);
    }

    // ============================================
    // get Tests
    // ============================================

    #[test]
    fn test_bytes_reader_get_success() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[10, 20, 30]);
        let reader = BytesReader::new(buf);

        assert_eq!(reader.get(0).unwrap(), 10);
        assert_eq!(reader.get(1).unwrap(), 20);
        assert_eq!(reader.get(2).unwrap(), 30);
    }

    #[test]
    fn test_bytes_reader_get_out_of_range() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[10, 20, 30]);
        let reader = BytesReader::new(buf);

        let result = reader.get(3);
        assert!(result.is_err());
        match result.unwrap_err().value {
            BytesReadErrorValue::IndexOutofRange => {}
            _ => panic!("Expected IndexOutofRange error"),
        }
    }

    #[test]
    fn test_bytes_reader_get_empty_buffer() {
        let reader = BytesReader::new(BytesMut::new());
        let result = reader.get(0);
        assert!(result.is_err());
    }

    // ============================================
    // extract_remaining_bytes and get_remaining_bytes Tests
    // ============================================

    #[test]
    fn test_bytes_reader_extract_remaining_bytes() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2, 3, 4, 5]);
        let mut reader = BytesReader::new(buf);

        // Read some bytes first
        reader.read_u8().unwrap();
        reader.read_u8().unwrap();

        let remaining = reader.extract_remaining_bytes();
        assert_eq!(&remaining[..], &[3, 4, 5]);
        assert!(reader.is_empty());
    }

    #[test]
    fn test_bytes_reader_get_remaining_bytes() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2, 3, 4, 5]);
        let mut reader = BytesReader::new(buf);

        reader.read_u8().unwrap();
        reader.read_u8().unwrap();

        let remaining = reader.get_remaining_bytes();
        assert_eq!(&remaining[..], &[3, 4, 5]);
        // Original buffer should still have data (clone)
        assert_eq!(reader.len(), 3);
    }

    // ============================================
    // Sequential Read Tests (Multiple types in sequence)
    // ============================================

    #[test]
    fn test_bytes_reader_sequential_reads() {
        let mut buf = BytesMut::new();
        // u8, u16, u32, u8
        buf.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        let mut reader = BytesReader::new(buf);

        assert_eq!(reader.read_u8().unwrap(), 0x01);
        assert_eq!(reader.read_u16::<BigEndian>().unwrap(), 0x0203);
        assert_eq!(reader.read_u32::<BigEndian>().unwrap(), 0x04050607);
        assert_eq!(reader.read_u8().unwrap(), 0x08);
        assert!(reader.is_empty());
    }

    // ============================================
    // Rc<RefCell<BytesReader>> Tests (Original tests preserved)
    // ============================================

    #[test]
    fn test_rc_refcell() {
        let reader = Rc::new(RefCell::new(BytesReader::new(BytesMut::new())));
        let xs: [u8; 3] = [1, 2, 3];
        reader.borrow_mut().extend_from_slice(&xs[..]);

        let mut rv = reader.borrow_mut().read_u8().unwrap();
        assert_eq!(rv, 1, "Incorrect value");

        rv = reader.borrow_mut().read_u8().unwrap();
        assert_eq!(rv, 2, "Incorrect value");

        rv = reader.borrow_mut().read_u8().unwrap();
        assert_eq!(rv, 3, "Incorrect value");
    }

    struct RefStruct {
        pub reader: Rc<RefCell<BytesReader>>,
    }

    impl RefStruct {
        pub fn new(reader: Rc<RefCell<BytesReader>>) -> Self {
            Self { reader }
        }

        pub fn extend_from_slice(&mut self, data: &[u8]) {
            self.reader.borrow_mut().extend_from_slice(data);
        }
    }

    #[test]
    fn test_struct_rc_refcell() {
        let reader = Rc::new(RefCell::new(BytesReader::new(BytesMut::new())));

        let mut ref_struct = RefStruct::new(reader);

        let xs: [u8; 3] = [1, 2, 3];
        ref_struct.extend_from_slice(&xs);

        let mut reader = ref_struct.reader.borrow_mut();

        let mut rv = reader.read_u8().unwrap();
        assert_eq!(rv, 1, "Incorrect value");

        rv = reader.read_u8().unwrap();
        assert_eq!(rv, 2, "Incorrect value");

        rv = reader.read_u8().unwrap();
        assert_eq!(rv, 3, "Incorrect value");
    }

    // ============================================
    // Property-based style tests (deterministic version)
    // ============================================

    #[test]
    fn test_bytes_reader_u8_roundtrip() {
        // Test all possible u8 values
        for i in 0..=255u8 {
            let mut buf = BytesMut::new();
            buf.extend_from_slice(&[i]);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u8().unwrap(), i);
        }
    }

    #[test]
    fn test_bytes_reader_u16_roundtrip_samples() {
        let test_values: [u16; 8] = [0, 1, 255, 256, 32767, 32768, 65534, 65535];
        for &val in &test_values {
            let mut buf = BytesMut::new();
            let mut bytes = [0u8; 2];
            byteorder::BigEndian::write_u16(&mut bytes, val);
            buf.extend_from_slice(&bytes);

            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u16::<BigEndian>().unwrap(), val);
        }
    }

    #[test]
    fn test_bytes_reader_u32_roundtrip_samples() {
        let test_values: [u32; 6] = [0, 1, 65535, 65536, 2147483647, u32::MAX];
        for &val in &test_values {
            let mut buf = BytesMut::new();
            let mut bytes = [0u8; 4];
            byteorder::BigEndian::write_u32(&mut bytes, val);
            buf.extend_from_slice(&bytes);

            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u32::<BigEndian>().unwrap(), val);
        }
    }

    #[test]
    fn test_bytes_reader_u64_roundtrip_samples() {
        let test_values: [u64; 4] = [0, 1, u32::MAX as u64 + 1, u64::MAX];
        for &val in &test_values {
            let mut buf = BytesMut::new();
            let mut bytes = [0u8; 8];
            byteorder::BigEndian::write_u64(&mut bytes, val);
            buf.extend_from_slice(&bytes);

            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u64::<BigEndian>().unwrap(), val);
        }
    }

    // ============================================
    // Property-based Tests (proptest)
    // ============================================

    #[test]
    fn test_bytes_reader_u8_property_roundtrip() {
        use proptest::prelude::*;
        proptest!(|(val in 0u8..=255u8)| {
            let mut buf = BytesMut::new();
            buf.extend_from_slice(&[val]);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u8().unwrap(), val);
        });
    }

    #[test]
    fn test_bytes_reader_u16_property_roundtrip_big_endian() {
        use proptest::prelude::*;
        proptest!(|(val in 0u16..=65535u16)| {
            let mut buf = BytesMut::new();
            let mut bytes = [0u8; 2];
            byteorder::BigEndian::write_u16(&mut bytes, val);
            buf.extend_from_slice(&bytes);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u16::<BigEndian>().unwrap(), val);
        });
    }

    #[test]
    fn test_bytes_reader_u16_property_roundtrip_little_endian() {
        use proptest::prelude::*;
        proptest!(|(val in 0u16..=65535u16)| {
            let mut buf = BytesMut::new();
            let mut bytes = [0u8; 2];
            byteorder::LittleEndian::write_u16(&mut bytes, val);
            buf.extend_from_slice(&bytes);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u16::<LittleEndian>().unwrap(), val);
        });
    }

    #[test]
    fn test_bytes_reader_u24_property_roundtrip_big_endian() {
        use proptest::prelude::*;
        proptest!(|(val in 0u32..=16777215u32)| {
            let mut buf = BytesMut::new();
            let mut bytes = [0u8; 3];
            byteorder::BigEndian::write_u24(&mut bytes, val);
            buf.extend_from_slice(&bytes);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u24::<BigEndian>().unwrap(), val);
        });
    }

    #[test]
    fn test_bytes_reader_u32_property_roundtrip_big_endian() {
        use proptest::prelude::*;
        proptest!(|(val in 0u32..=u32::MAX)| {
            let mut buf = BytesMut::new();
            let mut bytes = [0u8; 4];
            byteorder::BigEndian::write_u32(&mut bytes, val);
            buf.extend_from_slice(&bytes);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u32::<BigEndian>().unwrap(), val);
        });
    }

    #[test]
    fn test_bytes_reader_u32_property_roundtrip_little_endian() {
        use proptest::prelude::*;
        proptest!(|(val in 0u32..=u32::MAX)| {
            let mut buf = BytesMut::new();
            let mut bytes = [0u8; 4];
            byteorder::LittleEndian::write_u32(&mut bytes, val);
            buf.extend_from_slice(&bytes);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u32::<LittleEndian>().unwrap(), val);
        });
    }

    #[test]
    fn test_bytes_reader_u48_property_roundtrip_big_endian() {
        use proptest::prelude::*;
        proptest!(|(val in 0u64..=281474976710655u64)| {
            let mut buf = BytesMut::new();
            let mut bytes = [0u8; 6];
            byteorder::BigEndian::write_u48(&mut bytes, val);
            buf.extend_from_slice(&bytes);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u48::<BigEndian>().unwrap(), val);
        });
    }

    #[test]
    fn test_bytes_reader_u64_property_roundtrip_big_endian() {
        use proptest::prelude::*;
        proptest!(|(val in 0u64..=u64::MAX)| {
            let mut buf = BytesMut::new();
            let mut bytes = [0u8; 8];
            byteorder::BigEndian::write_u64(&mut bytes, val);
            buf.extend_from_slice(&bytes);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u64::<BigEndian>().unwrap(), val);
        });
    }

    #[test]
    fn test_bytes_reader_f64_property_roundtrip_big_endian() {
        use proptest::prelude::*;
        proptest!(|(val in proptest::num::f64::NORMAL)| {
            let mut buf = BytesMut::new();
            let mut bytes = [0u8; 8];
            byteorder::BigEndian::write_f64(&mut bytes, val);
            buf.extend_from_slice(&bytes);
            let mut reader = BytesReader::new(buf);
            let read_val = reader.read_f64::<BigEndian>().unwrap();
            // For floating point, we need to check if both are NaN or if they're close
            if val.is_nan() {
                assert!(read_val.is_nan());
            } else {
                assert!((read_val - val).abs() < f64::EPSILON * 100.0);
            }
        });
    }

    // ============================================
    // Additional Error Condition Tests
    // ============================================

    #[test]
    fn test_bytes_reader_read_u24_not_enough_bytes() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x01, 0x02]); // Only 2 bytes
        let mut reader = BytesReader::new(buf);
        let result = reader.read_u24::<BigEndian>();
        assert!(result.is_err());
        match result.unwrap_err().value {
            BytesReadErrorValue::NotEnoughBytes => {}
            _ => panic!("Expected NotEnoughBytes error"),
        }
    }

    #[test]
    fn test_bytes_reader_read_u48_not_enough_bytes() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05]); // Only 5 bytes
        let mut reader = BytesReader::new(buf);
        let result = reader.read_u48::<BigEndian>();
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_reader_read_f64_not_enough_bytes() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]); // Only 7 bytes
        let mut reader = BytesReader::new(buf);
        let result = reader.read_f64::<BigEndian>();
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_reader_get_boundary_conditions() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[10, 20, 30]);
        let reader = BytesReader::new(buf);

        // Valid indices
        assert_eq!(reader.get(0).unwrap(), 10);
        assert_eq!(reader.get(2).unwrap(), 30);

        // Boundary: exactly at length
        let result = reader.get(3);
        assert!(result.is_err());

        // Beyond boundary
        let result = reader.get(100);
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_reader_read_bytes_edge_cases() {
        // Empty buffer
        let mut reader = BytesReader::new(BytesMut::new());
        let result = reader.read_bytes(1);
        assert!(result.is_err());

        // Read exactly all bytes
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2, 3]);
        let mut reader = BytesReader::new(buf);
        let result = reader.read_bytes(3).unwrap();
        assert_eq!(&result[..], &[1, 2, 3]);
        assert!(reader.is_empty());
    }

    #[test]
    fn test_bytes_reader_advance_bytes_edge_cases() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2, 3, 4, 5]);
        let mut reader = BytesReader::new(buf);

        // Advance zero bytes
        let result = reader.advance_bytes(0).unwrap();
        assert!(result.is_empty());
        assert_eq!(reader.len(), 5);

        // Advance all bytes
        let result = reader.advance_bytes(5).unwrap();
        assert_eq!(&result[..], &[1, 2, 3, 4, 5]);
        assert_eq!(reader.len(), 5); // advance doesn't consume
    }

    #[test]
    fn test_bytes_reader_extract_remaining_bytes_edge_cases() {
        // Empty buffer
        let mut reader = BytesReader::new(BytesMut::new());
        let remaining = reader.extract_remaining_bytes();
        assert!(remaining.is_empty());
        assert!(reader.is_empty());

        // Full buffer
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2, 3]);
        let mut reader = BytesReader::new(buf);
        let remaining = reader.extract_remaining_bytes();
        assert_eq!(&remaining[..], &[1, 2, 3]);
        assert!(reader.is_empty());
    }

    #[test]
    fn test_bytes_reader_sequential_mixed_reads() {
        let mut buf = BytesMut::new();
        // u8, u16 (BE), u24 (BE), u32 (BE), u8
        buf.extend_from_slice(&[
            0x01, // u8
            0x02, 0x03, // u16
            0x04, 0x05, 0x06, // u24
            0x07, 0x08, 0x09, 0x0A, // u32
            0x0B, // u8
        ]);
        let mut reader = BytesReader::new(buf);

        assert_eq!(reader.read_u8().unwrap(), 0x01);
        assert_eq!(reader.read_u16::<BigEndian>().unwrap(), 0x0203);
        assert_eq!(reader.read_u24::<BigEndian>().unwrap(), 0x040506);
        assert_eq!(reader.read_u32::<BigEndian>().unwrap(), 0x0708090A);
        assert_eq!(reader.read_u8().unwrap(), 0x0B);
        assert!(reader.is_empty());
    }

    // ============================================
    // Cursor Method Tests
    // ============================================

    #[test]
    fn test_bytes_reader_read_bytes_cursor() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2, 3, 4, 5]);
        let mut reader = BytesReader::new(buf);

        let cursor = reader.read_bytes_cursor(3).unwrap();
        let inner = cursor.into_inner();
        assert_eq!(&inner[..], &[1, 2, 3]);
        assert_eq!(reader.len(), 2); // consumed 3 bytes
    }

    #[test]
    fn test_bytes_reader_read_bytes_cursor_not_enough() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2]);
        let mut reader = BytesReader::new(buf);

        let result = reader.read_bytes_cursor(5);
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_reader_advance_bytes_cursor() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2, 3, 4, 5]);
        let mut reader = BytesReader::new(buf);

        let cursor = reader.advance_bytes_cursor(3).unwrap();
        let inner = cursor.into_inner();
        assert_eq!(&inner[..], &[1, 2, 3]);
        assert_eq!(reader.len(), 5); // advance doesn't consume
    }

    #[test]
    fn test_bytes_reader_advance_bytes_cursor_not_enough() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1]);
        let mut reader = BytesReader::new(buf);

        let result = reader.advance_bytes_cursor(5);
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_reader_advance_u24_not_enough() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2]); // Only 2 bytes
        let mut reader = BytesReader::new(buf);

        let result = reader.advance_u24::<BigEndian>();
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_reader_read_u32_not_enough() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2, 3]); // Only 3 bytes
        let mut reader = BytesReader::new(buf);

        let result = reader.read_u32::<BigEndian>();
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_reader_read_u64_not_enough() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7]); // Only 7 bytes
        let mut reader = BytesReader::new(buf);

        let result = reader.read_u64::<BigEndian>();
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_reader_get_remaining_bytes_empty() {
        let reader = BytesReader::new(BytesMut::new());
        let remaining = reader.get_remaining_bytes();
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_bytes_reader_extend_after_partial_read() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1, 2, 3]);
        let mut reader = BytesReader::new(buf);

        // Read some bytes
        reader.read_u8().unwrap();

        // Extend with more data
        reader.extend_from_slice(&[4, 5, 6]);

        // Should have 5 bytes now (2 remaining + 3 new)
        assert_eq!(reader.len(), 5);
        assert_eq!(reader.read_u8().unwrap(), 2);
    }
}
