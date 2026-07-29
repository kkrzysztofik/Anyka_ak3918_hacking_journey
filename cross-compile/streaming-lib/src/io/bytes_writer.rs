use {
    super::{
        TNetIO,
        bytes_errors::{BytesWriteError, BytesWriteErrorValue},
    },
    byteorder::{ByteOrder, WriteBytesExt},
    bytes::BytesMut,
    rand,
    rand::RngExt,
    std::{io::Write, sync::Arc, time::Duration},
    tokio::{sync::Mutex, time::timeout},
};

pub struct BytesWriter {
    pub bytes: Vec<u8>,
}

impl Default for BytesWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl BytesWriter {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Start with room for `capacity` bytes already reserved.
    ///
    /// A writer that starts empty reallocates as it grows, copying everything written so far each
    /// time. On the RTP send path that happens once per datagram, so callers that know their final
    /// size say so up front.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }

    pub fn write_u8(&mut self, byte: u8) -> Result<(), BytesWriteError> {
        self.bytes.write_u8(byte)?;
        Ok(())
    }

    pub fn or_u8_at(&mut self, position: usize, byte: u8) -> Result<(), BytesWriteError> {
        if position >= self.bytes.len() {
            return Err(BytesWriteError {
                value: BytesWriteErrorValue::OutofIndex,
            });
        }
        self.bytes[position] |= byte;

        Ok(())
    }

    pub fn add_u8_at(&mut self, position: usize, byte: u8) -> Result<(), BytesWriteError> {
        if position >= self.bytes.len() {
            return Err(BytesWriteError {
                value: BytesWriteErrorValue::OutofIndex,
            });
        }
        self.bytes[position] = self.bytes[position].wrapping_add(byte);

        Ok(())
    }

    pub fn write_u8_at(&mut self, position: usize, byte: u8) -> Result<(), BytesWriteError> {
        if position >= self.bytes.len() {
            return Err(BytesWriteError {
                value: BytesWriteErrorValue::OutofIndex,
            });
        }
        self.bytes[position] = byte;

        Ok(())
    }

    pub fn get(&mut self, position: usize) -> Option<&u8> {
        self.bytes.get(position)
    }

    pub fn write_u16<T: ByteOrder>(&mut self, bytes: u16) -> Result<(), BytesWriteError> {
        self.bytes.write_u16::<T>(bytes)?;
        Ok(())
    }

    pub fn write_u24<T: ByteOrder>(&mut self, bytes: u32) -> Result<(), BytesWriteError> {
        self.bytes.write_u24::<T>(bytes)?;

        Ok(())
    }

    pub fn write_u32<T: ByteOrder>(&mut self, bytes: u32) -> Result<(), BytesWriteError> {
        self.bytes.write_u32::<T>(bytes)?;
        Ok(())
    }

    pub fn write_f64<T: ByteOrder>(&mut self, bytes: f64) -> Result<(), BytesWriteError> {
        self.bytes.write_f64::<T>(bytes)?;
        Ok(())
    }

    pub fn write_u64<T: ByteOrder>(&mut self, bytes: u64) -> Result<(), BytesWriteError> {
        self.bytes.write_u64::<T>(bytes)?;
        Ok(())
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<(), BytesWriteError> {
        self.bytes.write_all(buf)?;
        Ok(())
    }

    pub fn prepend(&mut self, buf: &[u8]) -> Result<(), BytesWriteError> {
        let tmp_bytes = self.bytes.clone();
        self.bytes.clear();
        self.bytes.write_all(buf)?;
        self.bytes.write_all(tmp_bytes.as_slice())?;
        Ok(())
    }

    pub fn append(&mut self, writer: &mut BytesWriter) {
        self.bytes.append(&mut writer.bytes);
    }

    pub fn write_random_bytes(&mut self, length: u32) -> Result<(), BytesWriteError> {
        let mut rng = rand::rng();
        for _ in 0..length {
            self.bytes.write_u8(rng.random())?;
        }
        Ok(())
    }
    pub fn extract_current_bytes(&mut self) -> BytesMut {
        let data = std::mem::take(&mut self.bytes);
        // `bytes` 1.x does not implement `From<Vec<u8>>` for `BytesMut`. `Bytes::from(Vec<u8>)`
        // reuses the `Vec` buffer when uniquely owned; `BytesMut::from(Bytes)` can then take that
        // allocation without copying when the `Bytes` handle is sole owner (see `bytes` crate).
        BytesMut::from(bytes::Bytes::from(data))
    }

    pub fn clear(&mut self) {
        self.bytes.clear();
    }

    pub fn get_current_bytes(&self) -> BytesMut {
        let mut rv_data = BytesMut::new();
        rv_data.extend_from_slice(&self.bytes[..]);
        rv_data
    }

    pub fn pop_bytes(&mut self, size: usize) {
        for _ in 0..size {
            self.bytes.pop();
        }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct AsyncBytesWriter {
    pub bytes_writer: BytesWriter,
    pub io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
}

impl AsyncBytesWriter {
    pub fn new(io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) -> Self {
        Self {
            bytes_writer: BytesWriter::new(),
            io,
        }
    }

    pub fn write_u8(&mut self, byte: u8) -> Result<(), BytesWriteError> {
        self.bytes_writer.write_u8(byte)
    }

    pub fn write_u16<T: ByteOrder>(&mut self, bytes: u16) -> Result<(), BytesWriteError> {
        self.bytes_writer.write_u16::<T>(bytes)
    }

    pub fn write_u24<T: ByteOrder>(&mut self, bytes: u32) -> Result<(), BytesWriteError> {
        self.bytes_writer.write_u24::<T>(bytes)
    }

    pub fn write_u32<T: ByteOrder>(&mut self, bytes: u32) -> Result<(), BytesWriteError> {
        self.bytes_writer.write_u32::<T>(bytes)
    }

    pub fn write_f64<T: ByteOrder>(&mut self, bytes: f64) -> Result<(), BytesWriteError> {
        self.bytes_writer.write_f64::<T>(bytes)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<(), BytesWriteError> {
        self.bytes_writer.write(buf)
    }

    pub fn write_random_bytes(&mut self, length: u32) -> Result<(), BytesWriteError> {
        self.bytes_writer.write_random_bytes(length)
    }

    pub fn extract_current_bytes(&mut self) -> BytesMut {
        self.bytes_writer.extract_current_bytes()
    }

    pub async fn flush(&mut self) -> Result<(), BytesWriteError> {
        if self.bytes_writer.bytes.is_empty() {
            return Ok(());
        }
        // Take the buffer to avoid an extra copy; on write error the pending data is dropped
        // (same as a failed flush after partial send).
        let buf = std::mem::take(&mut self.bytes_writer.bytes);
        let data = bytes::Bytes::from(buf);
        self.io.lock().await.write(data).await?;
        Ok(())
    }

    pub async fn flush_timeout(&mut self, duration: Duration) -> Result<(), BytesWriteError> {
        if self.bytes_writer.bytes.is_empty() {
            return Ok(());
        }
        let data = bytes::Bytes::copy_from_slice(&self.bytes_writer.bytes);
        let mut io = self.io.lock().await;
        let write_fut = io.write(data);
        match timeout(duration, write_fut).await {
            Ok(Ok(())) => {
                drop(io);
                self.bytes_writer.bytes.clear();
                Ok(())
            }
            Ok(Err(io_err)) => Err(BytesWriteError {
                value: BytesWriteErrorValue::BytesIOError(io_err),
            }),
            Err(_) => Err(BytesWriteError {
                value: BytesWriteErrorValue::Timeout,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{BigEndian, LittleEndian};
    use std::io::Write;

    // ============================================
    // BytesWriter Construction and Basic Operations
    // ============================================

    #[test]
    fn test_bytes_writer_new() {
        let writer = BytesWriter::new();
        assert_eq!(writer.len(), 0);
        assert!(writer.is_empty());
    }

    #[test]
    fn test_bytes_writer_default() {
        let writer = BytesWriter::default();
        assert_eq!(writer.len(), 0);
        assert!(writer.is_empty());
    }

    // ============================================
    // write_u8 Tests
    // ============================================

    #[test]
    fn test_bytes_writer_write_u8_success() {
        let mut writer = BytesWriter::new();
        writer.write_u8(0x42).unwrap();
        assert_eq!(writer.len(), 1);
        assert_eq!(writer.bytes[0], 0x42);
    }

    #[test]
    fn test_bytes_writer_write_u8_boundary_values() {
        let mut writer = BytesWriter::new();
        writer.write_u8(0x00).unwrap();
        writer.write_u8(0x7F).unwrap();
        writer.write_u8(0x80).unwrap();
        writer.write_u8(0xFF).unwrap();

        assert_eq!(writer.len(), 4);
        assert_eq!(writer.bytes, vec![0x00, 0x7F, 0x80, 0xFF]);
    }

    #[test]
    fn test_bytes_writer_write_u8_sequential() {
        let mut writer = BytesWriter::new();
        for i in 0..10u8 {
            writer.write_u8(i).unwrap();
        }
        assert_eq!(writer.len(), 10);
        assert_eq!(writer.bytes, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    // ============================================
    // write_u8_at, or_u8_at, add_u8_at Tests
    // ============================================

    #[test]
    fn test_bytes_writer_write_u8_at_success() {
        let mut writer = BytesWriter::new();
        writer.write_u8(0x00).unwrap();
        writer.write_u8(0x00).unwrap();
        writer.write_u8(0x00).unwrap();

        writer.write_u8_at(1, 0x42).unwrap();
        assert_eq!(writer.bytes, vec![0x00, 0x42, 0x00]);
    }

    #[test]
    fn test_bytes_writer_write_u8_at_out_of_index() {
        let mut writer = BytesWriter::new();
        writer.write_u8(0x00).unwrap();

        let result = writer.write_u8_at(5, 0x42);
        assert!(result.is_err());
        match result.unwrap_err().value {
            BytesWriteErrorValue::OutofIndex => {}
            _ => panic!("Expected OutofIndex error"),
        }
    }

    #[test]
    fn test_bytes_writer_or_u8_at_success() {
        let mut writer = BytesWriter::new();
        writer.write_u8(0b00001111).unwrap();

        writer.or_u8_at(0, 0b11110000).unwrap();
        assert_eq!(writer.bytes[0], 0b11111111);
    }

    #[test]
    fn test_bytes_writer_or_u8_at_out_of_index() {
        let mut writer = BytesWriter::new();
        let result = writer.or_u8_at(0, 0x42);
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_writer_add_u8_at_success() {
        let mut writer = BytesWriter::new();
        writer.write_u8(10).unwrap();

        writer.add_u8_at(0, 5).unwrap();
        assert_eq!(writer.bytes[0], 15);
    }

    #[test]
    fn test_bytes_writer_add_u8_at_out_of_index() {
        let mut writer = BytesWriter::new();
        let result = writer.add_u8_at(0, 5);
        assert!(result.is_err());
    }

    // ============================================
    // get Tests
    // ============================================

    #[test]
    fn test_bytes_writer_get_success() {
        let mut writer = BytesWriter::new();
        writer.write_u8(0x42).unwrap();
        writer.write_u8(0x43).unwrap();

        assert_eq!(writer.get(0), Some(&0x42));
        assert_eq!(writer.get(1), Some(&0x43));
    }

    #[test]
    fn test_bytes_writer_get_out_of_range() {
        let mut writer = BytesWriter::new();
        writer.write_u8(0x42).unwrap();

        assert_eq!(writer.get(1), None);
        assert_eq!(writer.get(100), None);
    }

    // ============================================
    // write_u16 Tests
    // ============================================

    #[test]
    fn test_bytes_writer_write_u16_big_endian() {
        let mut writer = BytesWriter::new();
        writer.write_u16::<BigEndian>(0x0102).unwrap();

        assert_eq!(writer.len(), 2);
        assert_eq!(writer.bytes, vec![0x01, 0x02]);
    }

    #[test]
    fn test_bytes_writer_write_u16_little_endian() {
        let mut writer = BytesWriter::new();
        writer.write_u16::<LittleEndian>(0x0102).unwrap();

        assert_eq!(writer.len(), 2);
        assert_eq!(writer.bytes, vec![0x02, 0x01]);
    }

    #[test]
    fn test_bytes_writer_write_u16_boundary_values() {
        // Min value
        let mut writer = BytesWriter::new();
        writer.write_u16::<BigEndian>(0).unwrap();
        assert_eq!(writer.bytes, vec![0x00, 0x00]);

        // Max value
        let mut writer = BytesWriter::new();
        writer.write_u16::<BigEndian>(0xFFFF).unwrap();
        assert_eq!(writer.bytes, vec![0xFF, 0xFF]);
    }

    // ============================================
    // write_u24 Tests
    // ============================================

    #[test]
    fn test_bytes_writer_write_u24_big_endian() {
        let mut writer = BytesWriter::new();
        writer.write_u24::<BigEndian>(0x010203).unwrap();

        assert_eq!(writer.len(), 3);
        assert_eq!(writer.bytes, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_bytes_writer_write_u24_little_endian() {
        let mut writer = BytesWriter::new();
        writer.write_u24::<LittleEndian>(0x010203).unwrap();

        assert_eq!(writer.len(), 3);
        assert_eq!(writer.bytes, vec![0x03, 0x02, 0x01]);
    }

    #[test]
    fn test_bytes_writer_write_u24_max_value() {
        let mut writer = BytesWriter::new();
        writer.write_u24::<BigEndian>(0xFFFFFF).unwrap();
        assert_eq!(writer.bytes, vec![0xFF, 0xFF, 0xFF]);
    }

    // ============================================
    // write_u32 Tests
    // ============================================

    #[test]
    fn test_bytes_writer_write_u32_big_endian() {
        let mut writer = BytesWriter::new();
        writer.write_u32::<BigEndian>(0x01020304).unwrap();

        assert_eq!(writer.len(), 4);
        assert_eq!(writer.bytes, vec![0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_bytes_writer_write_u32_little_endian() {
        let mut writer = BytesWriter::new();
        writer.write_u32::<LittleEndian>(0x01020304).unwrap();

        assert_eq!(writer.len(), 4);
        assert_eq!(writer.bytes, vec![0x04, 0x03, 0x02, 0x01]);
    }

    #[test]
    fn test_bytes_writer_write_u32_boundary_values() {
        // Min
        let mut writer = BytesWriter::new();
        writer.write_u32::<BigEndian>(0).unwrap();
        assert_eq!(writer.bytes, vec![0x00, 0x00, 0x00, 0x00]);

        // Max
        let mut writer = BytesWriter::new();
        writer.write_u32::<BigEndian>(u32::MAX).unwrap();
        assert_eq!(writer.bytes, vec![0xFF, 0xFF, 0xFF, 0xFF]);
    }

    // ============================================
    // write_u64 Tests
    // ============================================

    #[test]
    fn test_bytes_writer_write_u64_big_endian() {
        let mut writer = BytesWriter::new();
        writer.write_u64::<BigEndian>(0x0102030405060708).unwrap();

        assert_eq!(writer.len(), 8);
        assert_eq!(
            writer.bytes,
            vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        );
    }

    #[test]
    fn test_bytes_writer_write_u64_little_endian() {
        let mut writer = BytesWriter::new();
        writer
            .write_u64::<LittleEndian>(0x0102030405060708)
            .unwrap();

        assert_eq!(writer.len(), 8);
        assert_eq!(
            writer.bytes,
            vec![0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]
        );
    }

    // ============================================
    // write_f64 Tests
    // ============================================

    #[test]
    fn test_bytes_writer_write_f64_big_endian() {
        let mut writer = BytesWriter::new();
        writer.write_f64::<BigEndian>(1.0).unwrap();

        assert_eq!(writer.len(), 8);
        // IEEE 754 representation of 1.0
        assert_eq!(
            writer.bytes,
            vec![0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn test_bytes_writer_write_f64_pi() {
        let mut writer = BytesWriter::new();
        let pi: f64 = std::f64::consts::PI;
        writer.write_f64::<BigEndian>(pi).unwrap();

        assert_eq!(writer.len(), 8);
        // Verify by reading back
        let mut expected = [0u8; 8];
        byteorder::BigEndian::write_f64(&mut expected, pi);
        assert_eq!(writer.bytes, expected.to_vec());
    }

    // ============================================
    // write (byte slice) Tests
    // ============================================

    #[test]
    fn test_bytes_writer_write_slice() {
        let mut writer = BytesWriter::new();
        writer.write(&[1, 2, 3, 4, 5]).unwrap();

        assert_eq!(writer.len(), 5);
        assert_eq!(writer.bytes, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_bytes_writer_write_empty_slice() {
        let mut writer = BytesWriter::new();
        writer.write(&[]).unwrap();

        assert_eq!(writer.len(), 0);
        assert!(writer.is_empty());
    }

    #[test]
    fn test_bytes_writer_write_multiple_slices() {
        let mut writer = BytesWriter::new();
        writer.write(&[1, 2]).unwrap();
        writer.write(&[3, 4]).unwrap();
        writer.write(&[5]).unwrap();

        assert_eq!(writer.len(), 5);
        assert_eq!(writer.bytes, vec![1, 2, 3, 4, 5]);
    }

    // ============================================
    // prepend Tests
    // ============================================

    #[test]
    fn test_bytes_writer_prepend() {
        let mut writer = BytesWriter::new();
        writer.write(&[3, 4, 5]).unwrap();
        writer.prepend(&[1, 2]).unwrap();

        assert_eq!(writer.len(), 5);
        assert_eq!(writer.bytes, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_bytes_writer_prepend_to_empty() {
        let mut writer = BytesWriter::new();
        writer.prepend(&[1, 2, 3]).unwrap();

        assert_eq!(writer.len(), 3);
        assert_eq!(writer.bytes, vec![1, 2, 3]);
    }

    #[test]
    fn test_bytes_writer_prepend_empty() {
        let mut writer = BytesWriter::new();
        writer.write(&[1, 2, 3]).unwrap();
        writer.prepend(&[]).unwrap();

        assert_eq!(writer.len(), 3);
        assert_eq!(writer.bytes, vec![1, 2, 3]);
    }

    // ============================================
    // append Tests
    // ============================================

    #[test]
    fn test_bytes_writer_append() {
        let mut writer1 = BytesWriter::new();
        writer1.write(&[1, 2, 3]).unwrap();

        let mut writer2 = BytesWriter::new();
        writer2.write(&[4, 5, 6]).unwrap();

        writer1.append(&mut writer2);

        assert_eq!(writer1.len(), 6);
        assert_eq!(writer1.bytes, vec![1, 2, 3, 4, 5, 6]);
        assert!(writer2.is_empty());
    }

    #[test]
    fn test_bytes_writer_append_empty() {
        let mut writer1 = BytesWriter::new();
        writer1.write(&[1, 2, 3]).unwrap();

        let mut writer2 = BytesWriter::new();

        writer1.append(&mut writer2);

        assert_eq!(writer1.len(), 3);
        assert_eq!(writer1.bytes, vec![1, 2, 3]);
    }

    // ============================================
    // write_random_bytes Tests
    // ============================================

    #[test]
    fn test_bytes_writer_write_random_bytes_length() {
        let mut writer = BytesWriter::new();
        writer.write_random_bytes(10).unwrap();

        assert_eq!(writer.len(), 10);
    }

    #[test]
    fn test_bytes_writer_write_random_bytes_zero() {
        let mut writer = BytesWriter::new();
        writer.write_random_bytes(0).unwrap();

        assert_eq!(writer.len(), 0);
    }

    // ============================================
    // extract_current_bytes and get_current_bytes Tests
    // ============================================

    #[test]
    fn test_bytes_writer_extract_current_bytes() {
        let mut writer = BytesWriter::new();
        writer.write(&[1, 2, 3, 4, 5]).unwrap();

        let bytes = writer.extract_current_bytes();

        assert_eq!(&bytes[..], &[1, 2, 3, 4, 5]);
        assert!(writer.is_empty());
    }

    #[test]
    fn test_bytes_writer_get_current_bytes() {
        let mut writer = BytesWriter::new();
        writer.write(&[1, 2, 3, 4, 5]).unwrap();

        let bytes = writer.get_current_bytes();

        assert_eq!(&bytes[..], &[1, 2, 3, 4, 5]);
        // Original data should still be there
        assert_eq!(writer.len(), 5);
    }

    // ============================================
    // clear Tests
    // ============================================

    #[test]
    fn test_bytes_writer_clear() {
        let mut writer = BytesWriter::new();
        writer.write(&[1, 2, 3, 4, 5]).unwrap();
        assert_eq!(writer.len(), 5);

        writer.clear();

        assert!(writer.is_empty());
        assert_eq!(writer.len(), 0);
    }

    // ============================================
    // pop_bytes Tests
    // ============================================

    #[test]
    fn test_bytes_writer_pop_bytes() {
        let mut writer = BytesWriter::new();
        writer.write(&[1, 2, 3, 4, 5]).unwrap();

        writer.pop_bytes(2);

        assert_eq!(writer.len(), 3);
        assert_eq!(writer.bytes, vec![1, 2, 3]);
    }

    #[test]
    fn test_bytes_writer_pop_bytes_all() {
        let mut writer = BytesWriter::new();
        writer.write(&[1, 2, 3]).unwrap();

        writer.pop_bytes(3);

        assert!(writer.is_empty());
    }

    #[test]
    fn test_bytes_writer_pop_bytes_zero() {
        let mut writer = BytesWriter::new();
        writer.write(&[1, 2, 3]).unwrap();

        writer.pop_bytes(0);

        assert_eq!(writer.len(), 3);
    }

    // ============================================
    // Sequential Write Tests
    // ============================================

    #[test]
    fn test_bytes_writer_sequential_writes() {
        let mut writer = BytesWriter::new();
        writer.write_u8(0x01).unwrap();
        writer.write_u16::<BigEndian>(0x0203).unwrap();
        writer.write_u32::<BigEndian>(0x04050607).unwrap();
        writer.write_u8(0x08).unwrap();

        assert_eq!(writer.len(), 8);
        assert_eq!(
            writer.bytes,
            vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        );
    }

    // ============================================
    // Roundtrip Tests (Writer -> Reader)
    // ============================================

    #[test]
    fn test_bytes_writer_reader_u16_roundtrip() {
        use super::super::bytes_reader::BytesReader;
        use bytes::BytesMut;

        let test_values: [u16; 5] = [0, 1, 32767, 65534, 65535];

        for &val in &test_values {
            let mut writer = BytesWriter::new();
            writer.write_u16::<BigEndian>(val).unwrap();

            let mut buf = BytesMut::new();
            buf.extend_from_slice(&writer.bytes);
            let mut reader = BytesReader::new(buf);

            assert_eq!(reader.read_u16::<BigEndian>().unwrap(), val);
        }
    }

    #[test]
    fn test_bytes_writer_reader_u32_roundtrip() {
        use super::super::bytes_reader::BytesReader;
        use bytes::BytesMut;

        let test_values: [u32; 5] = [0, 1, 65535, 2147483647, u32::MAX];

        for &val in &test_values {
            let mut writer = BytesWriter::new();
            writer.write_u32::<BigEndian>(val).unwrap();

            let mut buf = BytesMut::new();
            buf.extend_from_slice(&writer.bytes);
            let mut reader = BytesReader::new(buf);

            assert_eq!(reader.read_u32::<BigEndian>().unwrap(), val);
        }
    }

    #[test]
    fn test_bytes_writer_reader_u64_roundtrip() {
        use super::super::bytes_reader::BytesReader;
        use bytes::BytesMut;

        let test_values: [u64; 4] = [0, 1, u32::MAX as u64 + 1, u64::MAX];

        for &val in &test_values {
            let mut writer = BytesWriter::new();
            writer.write_u64::<BigEndian>(val).unwrap();

            let mut buf = BytesMut::new();
            buf.extend_from_slice(&writer.bytes);
            let mut reader = BytesReader::new(buf);

            assert_eq!(reader.read_u64::<BigEndian>().unwrap(), val);
        }
    }

    #[test]
    fn test_bytes_writer_reader_f64_roundtrip() {
        use super::super::bytes_reader::BytesReader;
        use bytes::BytesMut;

        let test_values: [f64; 5] = [0.0, 1.0, -1.0, std::f64::consts::PI, f64::MAX];

        for &val in &test_values {
            let mut writer = BytesWriter::new();
            writer.write_f64::<BigEndian>(val).unwrap();

            let mut buf = BytesMut::new();
            buf.extend_from_slice(&writer.bytes);
            let mut reader = BytesReader::new(buf);

            let read_val = reader.read_f64::<BigEndian>().unwrap();
            assert!((read_val - val).abs() < f64::EPSILON || (val.is_nan() && read_val.is_nan()));
        }
    }

    // ============================================
    // Property-based Tests (proptest)
    // ============================================

    #[test]
    fn test_bytes_writer_u8_property_roundtrip() {
        use super::super::bytes_reader::BytesReader;
        use bytes::BytesMut;
        use proptest::prelude::*;

        proptest!(|(val in 0u8..=255u8)| {
            let mut writer = BytesWriter::new();
            writer.write_u8(val).unwrap();

            let mut buf = BytesMut::new();
            buf.extend_from_slice(&writer.bytes);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u8().unwrap(), val);
        });
    }

    #[test]
    fn test_bytes_writer_u16_property_roundtrip_big_endian() {
        use super::super::bytes_reader::BytesReader;
        use bytes::BytesMut;
        use proptest::prelude::*;

        proptest!(|(val in 0u16..=65535u16)| {
            let mut writer = BytesWriter::new();
            writer.write_u16::<BigEndian>(val).unwrap();

            let mut buf = BytesMut::new();
            buf.extend_from_slice(&writer.bytes);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u16::<BigEndian>().unwrap(), val);
        });
    }

    #[test]
    fn test_bytes_writer_u16_property_roundtrip_little_endian() {
        use super::super::bytes_reader::BytesReader;
        use bytes::BytesMut;
        use proptest::prelude::*;

        proptest!(|(val in 0u16..=65535u16)| {
            let mut writer = BytesWriter::new();
            writer.write_u16::<LittleEndian>(val).unwrap();

            let mut buf = BytesMut::new();
            buf.extend_from_slice(&writer.bytes);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u16::<LittleEndian>().unwrap(), val);
        });
    }

    #[test]
    fn test_bytes_writer_u24_property_roundtrip_big_endian() {
        use super::super::bytes_reader::BytesReader;
        use bytes::BytesMut;
        use proptest::prelude::*;

        proptest!(|(val in 0u32..=16777215u32)| {
            let mut writer = BytesWriter::new();
            writer.write_u24::<BigEndian>(val).unwrap();

            let mut buf = BytesMut::new();
            buf.extend_from_slice(&writer.bytes);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u24::<BigEndian>().unwrap(), val);
        });
    }

    #[test]
    fn test_bytes_writer_u32_property_roundtrip_big_endian() {
        use super::super::bytes_reader::BytesReader;
        use bytes::BytesMut;
        use proptest::prelude::*;

        proptest!(|(val in 0u32..=u32::MAX)| {
            let mut writer = BytesWriter::new();
            writer.write_u32::<BigEndian>(val).unwrap();

            let mut buf = BytesMut::new();
            buf.extend_from_slice(&writer.bytes);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u32::<BigEndian>().unwrap(), val);
        });
    }

    #[test]
    fn test_bytes_writer_u32_property_roundtrip_little_endian() {
        use super::super::bytes_reader::BytesReader;
        use bytes::BytesMut;
        use proptest::prelude::*;

        proptest!(|(val in 0u32..=u32::MAX)| {
            let mut writer = BytesWriter::new();
            writer.write_u32::<LittleEndian>(val).unwrap();

            let mut buf = BytesMut::new();
            buf.extend_from_slice(&writer.bytes);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u32::<LittleEndian>().unwrap(), val);
        });
    }

    #[test]
    fn test_bytes_writer_u64_property_roundtrip_big_endian() {
        use super::super::bytes_reader::BytesReader;
        use bytes::BytesMut;
        use proptest::prelude::*;

        proptest!(|(val in 0u64..=u64::MAX)| {
            let mut writer = BytesWriter::new();
            writer.write_u64::<BigEndian>(val).unwrap();

            let mut buf = BytesMut::new();
            buf.extend_from_slice(&writer.bytes);
            let mut reader = BytesReader::new(buf);
            assert_eq!(reader.read_u64::<BigEndian>().unwrap(), val);
        });
    }

    #[test]
    fn test_bytes_writer_f64_property_roundtrip_big_endian() {
        use super::super::bytes_reader::BytesReader;
        use bytes::BytesMut;
        use proptest::prelude::*;

        proptest!(|(val in proptest::num::f64::NORMAL)| {
            let mut writer = BytesWriter::new();
            writer.write_f64::<BigEndian>(val).unwrap();

            let mut buf = BytesMut::new();
            buf.extend_from_slice(&writer.bytes);
            let mut reader = BytesReader::new(buf);
            let read_val = reader.read_f64::<BigEndian>().unwrap();
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
    fn test_bytes_writer_write_u8_at_exact_boundary() {
        let mut writer = BytesWriter::new();
        writer.write_u8(0x00).unwrap();
        writer.write_u8(0x00).unwrap();

        // Valid: at index 1
        writer.write_u8_at(1, 0x42).unwrap();
        assert_eq!(writer.bytes[1], 0x42);

        // Invalid: at index 2 (beyond length)
        let result = writer.write_u8_at(2, 0x42);
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_writer_or_u8_at_bit_operations() {
        let mut writer = BytesWriter::new();
        writer.write_u8(0b00001111).unwrap();

        // OR with 0b11110000 should result in 0b11111111
        writer.or_u8_at(0, 0b11110000).unwrap();
        assert_eq!(writer.bytes[0], 0b11111111);

        // OR with 0b00000001 should keep it as 0b11111111
        writer.or_u8_at(0, 0b00000001).unwrap();
        assert_eq!(writer.bytes[0], 0b11111111);
    }

    #[test]
    fn test_bytes_writer_add_u8_at_overflow() {
        let mut writer = BytesWriter::new();
        writer.write_u8(250).unwrap();

        // Add 10 should wrap around (250 + 10 = 260 -> 4)
        writer.add_u8_at(0, 10).unwrap();
        assert_eq!(writer.bytes[0], 4);
    }

    #[test]
    fn test_bytes_writer_prepend_large_data() {
        let mut writer = BytesWriter::new();
        writer.write(&[3, 4, 5, 6, 7, 8, 9, 10]).unwrap();
        writer.prepend(&[1, 2]).unwrap();

        assert_eq!(writer.bytes, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn test_bytes_writer_append_multiple_times() {
        let mut writer1 = BytesWriter::new();
        writer1.write(&[1, 2]).unwrap();

        let mut writer2 = BytesWriter::new();
        writer2.write(&[3, 4]).unwrap();

        let mut writer3 = BytesWriter::new();
        writer3.write(&[5, 6]).unwrap();

        writer1.append(&mut writer2);
        writer1.append(&mut writer3);

        assert_eq!(writer1.bytes, vec![1, 2, 3, 4, 5, 6]);
        assert!(writer2.is_empty());
        assert!(writer3.is_empty());
    }

    #[test]
    fn test_bytes_writer_pop_bytes_edge_cases() {
        let mut writer = BytesWriter::new();
        writer.write(&[1, 2, 3]).unwrap();

        // Pop more than available (should panic or handle gracefully)
        // In current implementation, it just pops what's available
        writer.pop_bytes(5);
        assert!(writer.is_empty());
    }

    #[test]
    fn test_bytes_writer_extract_vs_get_current_bytes() {
        let mut writer = BytesWriter::new();
        writer.write(&[1, 2, 3, 4, 5]).unwrap();

        // get_current_bytes should not consume
        let bytes1 = writer.get_current_bytes();
        assert_eq!(&bytes1[..], &[1, 2, 3, 4, 5]);
        assert_eq!(writer.len(), 5);

        // extract_current_bytes should consume
        let bytes2 = writer.extract_current_bytes();
        assert_eq!(&bytes2[..], &[1, 2, 3, 4, 5]);
        assert!(writer.is_empty());
    }

    #[test]
    fn test_bytes_writer_sequential_mixed_writes() {
        let mut writer = BytesWriter::new();
        writer.write_u8(0x01).unwrap();
        writer.write_u16::<BigEndian>(0x0203).unwrap();
        writer.write_u24::<BigEndian>(0x040506).unwrap();
        writer.write_u32::<BigEndian>(0x0708090A).unwrap();
        writer.write_u64::<BigEndian>(0x0B0C0D0E0F101112).unwrap();
        writer.write_u8(0x19).unwrap();

        assert_eq!(
            writer.bytes,
            vec![
                0x01, // u8
                0x02, 0x03, // u16
                0x04, 0x05, 0x06, // u24
                0x07, 0x08, 0x09, 0x0A, // u32
                0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, // u64
                0x19, // u8
            ]
        );
    }

    // ============================================
    // Original Tests (Preserved)
    // ============================================

    #[test]
    fn test_write_vec() {
        let mut v: Vec<u8> = Vec::new();

        v.push(0x01);
        assert_eq!(1, v.len());
        assert_eq!(0x01, v[0]);

        v[0] = 0x02;
        assert_eq!(0x02, v[0]);

        const FLV_HEADER: [u8; 9] = [
            0x46, // 'F'
            0x4c, //'L'
            0x56, //'V'
            0x01, //version
            0x05, //00000101  audio tag  and video tag
            0x00, 0x00, 0x00, 0x09, //flv header size
        ];

        let rv = v.write(&FLV_HEADER);

        if let Ok(val) = rv {
            print!("{val} ");
        }

        assert_eq!(10, v.len());
    }

    #[test]
    fn test_bit_opertion() {
        let pts: i64 = 1627702096;

        let val = ((pts << 1) & 0xFE) as u8;

        println!("======={}=======", pts << 1);
        println!("======={val}=======");
    }

    #[test]
    fn test_bit_opertion2() {
        let flags = 0xC0;
        let pts: i64 = 1627702096;

        let b9 = ((flags >> 2) & 0x30)/* 0011/0010 */ | (((pts >> 30) & 0x07) << 1) as u8 /* PTS 30-32 */ | 0x01 /* marker_bit */;
        println!("=======b9{b9}=======");

        let b10 = (pts >> 22) as u8; /* PTS 22-29 */
        println!("=======b10{b10}=======");

        let b11 = ((pts >> 14) & 0xFE) as u8 /* PTS 15-21 */ | 0x01; /* marker_bit */
        println!("=======b11{b11}=======");

        let b12 = (pts >> 7) as u8; /* PTS 7-14 */
        println!("=======b12{b12}=======");

        let b13 = ((pts << 1) & 0xFE) as u8 /* PTS 0-6 */ | 0x01; /* marker_bit */
        println!("=======b13{b13}=======");
    }

    #[test]
    fn test_bit_opertion3() {
        let pts: i64 = 1627702096;

        let b12 = ((pts & 0x7fff) << 1) | 1; /* PTS 7-14 */
        println!("=======b12{}=======", b12 >> 8_u8);
        println!("=======b13{}=======", b12 as u8);
    }

    // ============================================
    // Additional Edge Case Tests for Coverage
    // ============================================

    #[test]
    fn test_bytes_writer_add_u8_at_wrapping() {
        let mut writer = BytesWriter::new();
        writer.write_u8(250).unwrap();

        // Add 10 to 250, should wrap to 4 (250 + 10 = 260, 260 % 256 = 4)
        writer.add_u8_at(0, 10).unwrap();
        assert_eq!(writer.bytes[0], 4);
    }

    #[test]
    fn test_bytes_writer_or_u8_at_no_change() {
        let mut writer = BytesWriter::new();
        writer.write_u8(0xFF).unwrap();

        // OR with anything on 0xFF stays 0xFF
        writer.or_u8_at(0, 0x42).unwrap();
        assert_eq!(writer.bytes[0], 0xFF);
    }

    #[test]
    fn test_bytes_writer_write_u8_at_first_position() {
        let mut writer = BytesWriter::new();
        writer.write(&[0x01, 0x02, 0x03]).unwrap();

        writer.write_u8_at(0, 0xFF).unwrap();
        assert_eq!(writer.bytes, vec![0xFF, 0x02, 0x03]);
    }

    #[test]
    fn test_bytes_writer_write_u8_at_last_position() {
        let mut writer = BytesWriter::new();
        writer.write(&[0x01, 0x02, 0x03]).unwrap();

        writer.write_u8_at(2, 0xFF).unwrap();
        assert_eq!(writer.bytes, vec![0x01, 0x02, 0xFF]);
    }

    #[test]
    fn test_bytes_writer_get_empty() {
        let mut writer = BytesWriter::new();
        assert_eq!(writer.get(0), None);
    }

    #[test]
    fn test_bytes_writer_pop_bytes_more_than_available() {
        let mut writer = BytesWriter::new();
        writer.write(&[1, 2, 3]).unwrap();

        // Pop more than available - should just pop until empty
        writer.pop_bytes(10);
        assert!(writer.is_empty());
    }

    #[test]
    fn test_bytes_writer_extract_empty() {
        let mut writer = BytesWriter::new();
        let bytes = writer.extract_current_bytes();
        assert!(bytes.is_empty());
    }

    #[test]
    fn test_bytes_writer_get_current_bytes_empty() {
        let writer = BytesWriter::new();
        let bytes = writer.get_current_bytes();
        assert!(bytes.is_empty());
    }

    #[test]
    fn test_bytes_writer_clear_empty() {
        let mut writer = BytesWriter::new();
        writer.clear(); // Should not panic
        assert!(writer.is_empty());
    }

    #[test]
    fn test_bytes_writer_prepend_multiple() {
        let mut writer = BytesWriter::new();
        writer.write(&[5, 6]).unwrap();
        writer.prepend(&[3, 4]).unwrap();
        writer.prepend(&[1, 2]).unwrap();

        assert_eq!(writer.bytes, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_bytes_writer_append_to_empty() {
        let mut writer1 = BytesWriter::new();
        let mut writer2 = BytesWriter::new();
        writer2.write(&[1, 2, 3]).unwrap();

        writer1.append(&mut writer2);

        assert_eq!(writer1.bytes, vec![1, 2, 3]);
        assert!(writer2.is_empty());
    }

    // ============================================
    // AsyncBytesWriter Tests
    // ============================================

    use crate::io::NetType;
    use crate::io::bytesio_errors::BytesIOError;
    use async_trait::async_trait;
    use bytes::Bytes;
    use mockall::mock;

    mock! {
        NetIO {}

        #[async_trait]
        impl TNetIO for NetIO {
            async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError>;
            async fn read(&mut self) -> Result<BytesMut, BytesIOError>;
            async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError>;
            fn get_net_type(&self) -> NetType;
        }
    }

    #[tokio::test]
    async fn test_async_bytes_writer_new() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let writer = AsyncBytesWriter::new(mock_io);
        assert!(writer.bytes_writer.is_empty());
    }

    #[tokio::test]
    async fn test_async_bytes_writer_write_u8() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut writer = AsyncBytesWriter::new(mock_io);
        writer.write_u8(0x42).unwrap();
        assert_eq!(writer.bytes_writer.len(), 1);
    }

    #[tokio::test]
    async fn test_async_bytes_writer_write_u16() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut writer = AsyncBytesWriter::new(mock_io);
        writer.write_u16::<BigEndian>(0x1234).unwrap();
        assert_eq!(writer.bytes_writer.len(), 2);
    }

    #[tokio::test]
    async fn test_async_bytes_writer_write_u24() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut writer = AsyncBytesWriter::new(mock_io);
        writer.write_u24::<BigEndian>(0x123456).unwrap();
        assert_eq!(writer.bytes_writer.len(), 3);
    }

    #[tokio::test]
    async fn test_async_bytes_writer_write_u32() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut writer = AsyncBytesWriter::new(mock_io);
        writer.write_u32::<BigEndian>(0x12345678).unwrap();
        assert_eq!(writer.bytes_writer.len(), 4);
    }

    #[tokio::test]
    async fn test_async_bytes_writer_write_f64() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut writer = AsyncBytesWriter::new(mock_io);
        writer.write_f64::<BigEndian>(std::f64::consts::PI).unwrap();
        assert_eq!(writer.bytes_writer.len(), 8);
    }

    #[tokio::test]
    async fn test_async_bytes_writer_write() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut writer = AsyncBytesWriter::new(mock_io);
        writer.write(&[1, 2, 3, 4, 5]).unwrap();
        assert_eq!(writer.bytes_writer.len(), 5);
    }

    #[tokio::test]
    async fn test_async_bytes_writer_write_random_bytes() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut writer = AsyncBytesWriter::new(mock_io);
        writer.write_random_bytes(10).unwrap();
        assert_eq!(writer.bytes_writer.len(), 10);
    }

    #[tokio::test]
    async fn test_async_bytes_writer_extract_current_bytes() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut writer = AsyncBytesWriter::new(mock_io);
        writer.write(&[1, 2, 3]).unwrap();

        let bytes = writer.extract_current_bytes();
        assert_eq!(bytes.len(), 3);
        assert!(writer.bytes_writer.is_empty());
    }

    #[tokio::test]
    async fn test_async_bytes_writer_flush() {
        let mut mock_io = MockNetIO::new();
        mock_io.expect_write().times(1).returning(|_| Ok(()));

        let io = Arc::new(Mutex::new(
            Box::new(mock_io) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut writer = AsyncBytesWriter::new(io);
        writer.write(&[1, 2, 3]).unwrap();

        let result = writer.flush().await;
        assert!(result.is_ok());
        assert!(writer.bytes_writer.is_empty());
    }

    #[tokio::test]
    async fn test_async_bytes_writer_flush_timeout_success() {
        let mut mock_io = MockNetIO::new();
        mock_io.expect_write().times(1).returning(|_| Ok(()));

        let io = Arc::new(Mutex::new(
            Box::new(mock_io) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut writer = AsyncBytesWriter::new(io);
        writer.write(&[1, 2, 3]).unwrap();

        let result = writer.flush_timeout(Duration::from_secs(5)).await;
        assert!(result.is_ok());
        assert!(writer.bytes_writer.is_empty());
    }
}
