use {
    super::{
        Amf0IndexMap, Amf0ValueType, Amf0WriteError, amf0_markers, errors::Amf0WriteErrorValue,
    },
    crate::bytesio::bytes_writer::BytesWriter,
    byteorder::BigEndian,
    bytes::BytesMut,
};

#[derive(Default)]
pub struct Amf0Writer {
    writer: BytesWriter,
}

impl Amf0Writer {
    pub fn new() -> Self {
        Self {
            writer: BytesWriter::new(),
        }
    }
    pub fn write_anys(&mut self, values: &Vec<Amf0ValueType>) -> Result<(), Amf0WriteError> {
        for val in values {
            self.write_any(val)?;
        }

        Ok(())
    }
    pub fn write_any(&mut self, value: &Amf0ValueType) -> Result<(), Amf0WriteError> {
        match *value {
            Amf0ValueType::Boolean(ref val) => self.write_bool(val),
            Amf0ValueType::Null => self.write_null(),
            Amf0ValueType::Number(ref val) => self.write_number(val),
            Amf0ValueType::UTF8String(ref val) => self.write_string(val),
            Amf0ValueType::LongUTF8String(ref val) => self.write_long_string(val),
            Amf0ValueType::Object(ref val) => self.write_object(val),
            Amf0ValueType::EcmaArray(ref val) => self.write_ecma_array(val),
            Amf0ValueType::END => self.write_object_eof(),
        }
    }

    pub fn write_number(&mut self, value: &f64) -> Result<(), Amf0WriteError> {
        self.writer.write_u8(amf0_markers::NUMBER)?;
        self.writer.write_f64::<BigEndian>(*value)?;
        Ok(())
    }

    pub fn write_bool(&mut self, value: &bool) -> Result<(), Amf0WriteError> {
        self.writer.write_u8(amf0_markers::BOOLEAN)?;
        self.writer.write_u8(*value as u8)?;
        Ok(())
    }

    pub fn write_string(&mut self, value: &String) -> Result<(), Amf0WriteError> {
        if value.len() > (u16::MAX as usize) {
            return Err(Amf0WriteError(Amf0WriteErrorValue::NormalStringTooLong));
        }

        self.writer.write_u8(amf0_markers::STRING)?;
        self.writer.write_u16::<BigEndian>(value.len() as u16)?;
        self.writer.write(value.as_bytes())?;

        Ok(())
    }

    pub fn write_long_string(&mut self, value: &String) -> Result<(), Amf0WriteError> {
        if value.len() > (u32::MAX as usize) {
            return Err(Amf0WriteError(Amf0WriteErrorValue::LongStringTooLong));
        }

        self.writer.write_u8(amf0_markers::LONG_STRING)?;
        self.writer.write_u32::<BigEndian>(value.len() as u32)?;
        self.writer.write(value.as_bytes())?;

        Ok(())
    }

    pub fn write_null(&mut self) -> Result<(), Amf0WriteError> {
        self.writer.write_u8(amf0_markers::NULL)?;
        Ok(())
    }

    pub fn write_object_eof(&mut self) -> Result<(), Amf0WriteError> {
        self.writer
            .write_u24::<BigEndian>(amf0_markers::OBJECT_END as u32)?;
        Ok(())
    }

    pub fn write_object(&mut self, properties: &Amf0IndexMap) -> Result<(), Amf0WriteError> {
        self.writer.write_u8(amf0_markers::OBJECT)?;

        for (key, value) in properties {
            self.writer.write_u16::<BigEndian>(key.len() as u16)?;
            self.writer.write(key.as_bytes())?;
            self.write_any(value)?;
        }

        self.write_object_eof()?;
        Ok(())
    }

    pub fn write_ecma_array(&mut self, properties: &Amf0IndexMap) -> Result<(), Amf0WriteError> {
        self.writer.write_u8(amf0_markers::ECMA_ARRAY)?;
        self.writer
            .write_u32::<BigEndian>(properties.len() as u32)?;

        for (key, value) in properties {
            self.writer.write_u16::<BigEndian>(key.len() as u16)?;
            self.writer.write(key.as_bytes())?;
            self.write_any(value)?;
        }

        self.write_object_eof()?;
        Ok(())
    }

    // pub async fn flush(&mut self) -> Result<(), Amf0WriteError> {
    //     self.writer.flush()?;
    // }

    pub fn extract_current_bytes(&mut self) -> BytesMut {
        self.writer.extract_current_bytes()
    }

    pub fn get_current_bytes(&mut self) -> BytesMut {
        self.writer.get_current_bytes()
    }

    pub fn len(&self) -> usize {
        self.writer.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytesio::bytes_reader::BytesReader;
    use crate::container::amf0::amf0_reader::Amf0Reader;

    // ============================================
    // Construction Tests
    // ============================================

    #[test]
    fn test_amf0_writer_new() {
        let writer = Amf0Writer::new();
        assert_eq!(writer.len(), 0);
        assert!(writer.is_empty());
    }

    #[test]
    fn test_amf0_writer_default() {
        let writer = Amf0Writer::default();
        assert_eq!(writer.len(), 0);
        assert!(writer.is_empty());
    }

    // ============================================
    // Number Tests
    // ============================================

    #[test]
    fn test_write_number() {
        let mut writer = Amf0Writer::new();
        writer.write_number(&123.456).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes.len(), 9); // marker (1) + f64 (8)
        assert_eq!(bytes[0], amf0_markers::NUMBER);
    }

    #[test]
    fn test_write_number_zero() {
        let mut writer = Amf0Writer::new();
        writer.write_number(&0.0).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::NUMBER);
    }

    #[test]
    fn test_write_number_negative() {
        let mut writer = Amf0Writer::new();
        writer.write_number(&-123.456).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::NUMBER);
    }

    #[test]
    fn test_write_number_max() {
        let mut writer = Amf0Writer::new();
        writer.write_number(&f64::MAX).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::NUMBER);
    }

    // ============================================
    // Boolean Tests
    // ============================================

    #[test]
    fn test_write_bool_true() {
        let mut writer = Amf0Writer::new();
        writer.write_bool(&true).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes[0], amf0_markers::BOOLEAN);
        assert_eq!(bytes[1], 1);
    }

    #[test]
    fn test_write_bool_false() {
        let mut writer = Amf0Writer::new();
        writer.write_bool(&false).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes[0], amf0_markers::BOOLEAN);
        assert_eq!(bytes[1], 0);
    }

    // ============================================
    // String Tests
    // ============================================

    #[test]
    fn test_write_string() {
        let mut writer = Amf0Writer::new();
        let test_string = "Hello, World!";
        writer.write_string(&test_string.to_string()).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::STRING);
        assert_eq!(bytes[1], 0x00);
        assert_eq!(bytes[2], test_string.len() as u8);
        assert_eq!(&bytes[3..], test_string.as_bytes());
    }

    #[test]
    fn test_write_string_empty() {
        let mut writer = Amf0Writer::new();
        writer.write_string(&String::new()).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::STRING);
        assert_eq!(&bytes[1..3], &[0x00, 0x00]); // Length = 0
    }

    #[test]
    fn test_write_string_max_length() {
        let mut writer = Amf0Writer::new();
        let test_string = "a".repeat(u16::MAX as usize);
        writer.write_string(&test_string).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::STRING);
        assert_eq!(&bytes[1..3], &[0xFF, 0xFF]); // Max u16 length
    }

    #[test]
    fn test_write_string_too_long() {
        let mut writer = Amf0Writer::new();
        let test_string = "a".repeat((u16::MAX as usize) + 1);
        let result = writer.write_string(&test_string);
        assert!(result.is_err());
        match &result.unwrap_err().0 {
            Amf0WriteErrorValue::NormalStringTooLong => {}
            _ => panic!("Expected NormalStringTooLong error"),
        }
    }

    // ============================================
    // Null Tests
    // ============================================

    #[test]
    fn test_write_null() {
        let mut writer = Amf0Writer::new();
        writer.write_null().unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes.len(), 1);
        assert_eq!(bytes[0], amf0_markers::NULL);
    }

    // ============================================
    // Object Tests
    // ============================================

    #[test]
    fn test_write_object_empty() {
        let mut writer = Amf0Writer::new();
        let props = Amf0IndexMap::default();
        writer.write_object(&props).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::OBJECT);
        assert_eq!(&bytes[1..4], &[0x00, 0x00, 0x09]); // OBJECT_END marker
    }

    #[test]
    fn test_write_object_with_properties() {
        let mut writer = Amf0Writer::new();
        let mut props = Amf0IndexMap::default();
        props.insert(
            "key1".to_string(),
            Amf0ValueType::UTF8String("value1".to_string()),
        );
        props.insert("key2".to_string(), Amf0ValueType::Number(42.0));
        writer.write_object(&props).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::OBJECT);
        // Should end with OBJECT_END
        assert!(bytes.len() > 4);
    }

    #[test]
    fn test_write_object_nested() {
        let mut writer = Amf0Writer::new();
        let mut outer_props = Amf0IndexMap::default();
        let mut inner_props = Amf0IndexMap::default();
        inner_props.insert(
            "inner_key".to_string(),
            Amf0ValueType::UTF8String("inner_value".to_string()),
        );
        outer_props.insert("nested".to_string(), Amf0ValueType::Object(inner_props));
        writer.write_object(&outer_props).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::OBJECT);
    }

    // ============================================
    // ECMA Array Tests
    // ============================================

    #[test]
    fn test_write_ecma_array_empty() {
        let mut writer = Amf0Writer::new();
        let props = Amf0IndexMap::default();
        writer.write_ecma_array(&props).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::ECMA_ARRAY);
        assert_eq!(&bytes[1..5], &[0x00, 0x00, 0x00, 0x00]); // Count = 0
        assert_eq!(&bytes[5..8], &[0x00, 0x00, 0x09]); // OBJECT_END
    }

    #[test]
    fn test_write_ecma_array_with_elements() {
        let mut writer = Amf0Writer::new();
        let mut props = Amf0IndexMap::default();
        props.insert(
            "0".to_string(),
            Amf0ValueType::UTF8String("first".to_string()),
        );
        props.insert(
            "1".to_string(),
            Amf0ValueType::UTF8String("second".to_string()),
        );
        writer.write_ecma_array(&props).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::ECMA_ARRAY);
        assert_eq!(&bytes[1..5], &[0x00, 0x00, 0x00, 0x02]); // Count = 2
    }

    // ============================================
    // write_any Tests
    // ============================================

    #[test]
    fn test_write_any_number() {
        let mut writer = Amf0Writer::new();
        writer.write_any(&Amf0ValueType::Number(42.0)).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::NUMBER);
    }

    #[test]
    fn test_write_any_boolean() {
        let mut writer = Amf0Writer::new();
        writer.write_any(&Amf0ValueType::Boolean(true)).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::BOOLEAN);
    }

    #[test]
    fn test_write_any_string() {
        let mut writer = Amf0Writer::new();
        writer
            .write_any(&Amf0ValueType::UTF8String("test".to_string()))
            .unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::STRING);
    }

    #[test]
    fn test_write_any_null() {
        let mut writer = Amf0Writer::new();
        writer.write_any(&Amf0ValueType::Null).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::NULL);
    }

    #[test]
    fn test_write_any_object() {
        let mut writer = Amf0Writer::new();
        let mut props = Amf0IndexMap::default();
        props.insert(
            "key".to_string(),
            Amf0ValueType::UTF8String("value".to_string()),
        );
        writer.write_any(&Amf0ValueType::Object(props)).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::OBJECT);
    }

    #[test]
    fn test_write_any_ecma_array() {
        let mut writer = Amf0Writer::new();
        let mut props = Amf0IndexMap::default();
        props.insert("0".to_string(), Amf0ValueType::Number(1.0));
        writer.write_any(&Amf0ValueType::EcmaArray(props)).unwrap();

        let bytes = writer.get_current_bytes();
        assert_eq!(bytes[0], amf0_markers::ECMA_ARRAY);
    }

    // ============================================
    // write_anys Tests
    // ============================================

    #[test]
    fn test_write_anys_multiple() {
        let mut writer = Amf0Writer::new();
        let values = vec![
            Amf0ValueType::UTF8String("hello".to_string()),
            Amf0ValueType::Number(42.0),
            Amf0ValueType::Boolean(true),
        ];
        writer.write_anys(&values).unwrap();

        let bytes = writer.get_current_bytes();
        assert!(bytes.len() > 10); // Should have multiple values
    }

    // ============================================
    // Round-trip Tests (Writer -> Reader)
    // ============================================

    #[test]
    fn test_amf0_writer_reader_roundtrip_number() {
        let value = 123.456;
        let mut writer = Amf0Writer::new();
        writer.write_number(&value).unwrap();

        let written_bytes = writer.extract_current_bytes();
        let bytes_reader = BytesReader::new(written_bytes);
        let mut amf_reader = Amf0Reader::new(bytes_reader);
        let read_value = amf_reader.read_with_type(amf0_markers::NUMBER).unwrap();
        assert_eq!(read_value, Amf0ValueType::Number(value));
    }

    #[test]
    fn test_amf0_writer_reader_roundtrip_boolean() {
        let value = true;
        let mut writer = Amf0Writer::new();
        writer.write_bool(&value).unwrap();

        let written_bytes = writer.extract_current_bytes();
        let bytes_reader = BytesReader::new(written_bytes);
        let mut amf_reader = Amf0Reader::new(bytes_reader);
        let read_value = amf_reader.read_with_type(amf0_markers::BOOLEAN).unwrap();
        assert_eq!(read_value, Amf0ValueType::Boolean(value));
    }

    #[test]
    fn test_amf0_writer_reader_roundtrip_string() {
        let value = "test_string_123".to_string();
        let mut writer = Amf0Writer::new();
        writer.write_string(&value).unwrap();

        let written_bytes = writer.extract_current_bytes();
        let bytes_reader = BytesReader::new(written_bytes);
        let mut amf_reader = Amf0Reader::new(bytes_reader);
        let read_value = amf_reader.read_with_type(amf0_markers::STRING).unwrap();
        assert_eq!(read_value, Amf0ValueType::UTF8String(value));
    }

    #[test]
    fn test_amf0_writer_reader_roundtrip_object() {
        let mut props = Amf0IndexMap::default();
        props.insert(
            "key1".to_string(),
            Amf0ValueType::UTF8String("value1".to_string()),
        );
        props.insert("key2".to_string(), Amf0ValueType::Number(42.0));
        props.insert("key3".to_string(), Amf0ValueType::Boolean(true));

        let mut writer = Amf0Writer::new();
        writer.write_object(&props).unwrap();

        let written_bytes = writer.extract_current_bytes();
        let bytes_reader = BytesReader::new(written_bytes);
        let mut amf_reader = Amf0Reader::new(bytes_reader);
        let read_value = amf_reader.read_with_type(amf0_markers::OBJECT).unwrap();

        if let Amf0ValueType::Object(read_props) = read_value {
            assert_eq!(read_props.len(), 3);
            assert_eq!(
                read_props.get("key1"),
                Some(&Amf0ValueType::UTF8String("value1".to_string()))
            );
            assert_eq!(read_props.get("key2"), Some(&Amf0ValueType::Number(42.0)));
            assert_eq!(read_props.get("key3"), Some(&Amf0ValueType::Boolean(true)));
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_amf0_writer_reader_roundtrip_ecma_array() {
        let mut props = Amf0IndexMap::default();
        props.insert(
            "0".to_string(),
            Amf0ValueType::UTF8String("first".to_string()),
        );
        props.insert(
            "1".to_string(),
            Amf0ValueType::UTF8String("second".to_string()),
        );

        let mut writer = Amf0Writer::new();
        writer.write_ecma_array(&props).unwrap();

        let written_bytes = writer.extract_current_bytes();
        let bytes_reader = BytesReader::new(written_bytes);
        let mut amf_reader = Amf0Reader::new(bytes_reader);
        let read_value = amf_reader.read_with_type(amf0_markers::ECMA_ARRAY).unwrap();

        if let Amf0ValueType::EcmaArray(read_props) = read_value {
            assert_eq!(read_props.len(), 2);
            assert_eq!(
                read_props.get("0"),
                Some(&Amf0ValueType::UTF8String("first".to_string()))
            );
            assert_eq!(
                read_props.get("1"),
                Some(&Amf0ValueType::UTF8String("second".to_string()))
            );
        } else {
            panic!("Expected EcmaArray");
        }
    }

    #[test]
    fn test_amf0_writer_reader_roundtrip_nested_object() {
        let mut inner_props = Amf0IndexMap::default();
        inner_props.insert(
            "inner_key".to_string(),
            Amf0ValueType::UTF8String("inner_value".to_string()),
        );

        let mut outer_props = Amf0IndexMap::default();
        outer_props.insert(
            "nested".to_string(),
            Amf0ValueType::Object(inner_props.clone()),
        );

        let mut writer = Amf0Writer::new();
        writer.write_object(&outer_props).unwrap();

        let written_bytes = writer.extract_current_bytes();
        let bytes_reader = BytesReader::new(written_bytes);
        let mut amf_reader = Amf0Reader::new(bytes_reader);
        let read_value = amf_reader.read_with_type(amf0_markers::OBJECT).unwrap();

        if let Amf0ValueType::Object(read_outer) = read_value {
            if let Some(Amf0ValueType::Object(read_inner)) = read_outer.get("nested") {
                assert_eq!(
                    read_inner.get("inner_key"),
                    Some(&Amf0ValueType::UTF8String("inner_value".to_string()))
                );
            } else {
                panic!("Expected nested Object");
            }
        } else {
            panic!("Expected Object");
        }
    }

    // ============================================
    // Property-based Tests (proptest)
    // ============================================

    #[test]
    fn test_amf0_writer_reader_property_roundtrip_number() {
        use proptest::prelude::*;
        proptest!(|(val in proptest::num::f64::NORMAL)| {
            let mut writer = Amf0Writer::new();
            writer.write_number(&val).unwrap();

            let written_bytes = writer.extract_current_bytes();
            let bytes_reader = BytesReader::new(written_bytes);
            let mut amf_reader = Amf0Reader::new(bytes_reader);
            let read_value = amf_reader.read_with_type(amf0_markers::NUMBER).unwrap();

            if let Amf0ValueType::Number(read_val) = read_value {
                if val.is_nan() {
                    assert!(read_val.is_nan());
                } else {
                    assert!((read_val - val).abs() < f64::EPSILON * 100.0);
                }
            } else {
                panic!("Expected Number");
            }
        });
    }

    #[test]
    fn test_amf0_writer_reader_property_roundtrip_string() {
        use proptest::prelude::*;
        proptest!(|(val in ".*")| {
            if val.len() <= u16::MAX as usize {
                let mut writer = Amf0Writer::new();
                writer.write_string(&val).unwrap();

                let written_bytes = writer.extract_current_bytes();
                let bytes_reader = BytesReader::new(written_bytes);
                let mut amf_reader = Amf0Reader::new(bytes_reader);
                let read_value = amf_reader.read_with_type(amf0_markers::STRING).unwrap();

                assert_eq!(read_value, Amf0ValueType::UTF8String(val));
            }
        });
    }

    // ============================================
    // Buffer Management Tests
    // ============================================

    #[test]
    fn test_extract_vs_get_current_bytes() {
        let mut writer = Amf0Writer::new();
        writer.write_number(&42.0).unwrap();

        // get_current_bytes should not consume
        let bytes1 = writer.get_current_bytes();
        assert_eq!(bytes1.len(), 9);
        assert_eq!(writer.len(), 9);

        // extract_current_bytes should consume
        let bytes2 = writer.extract_current_bytes();
        assert_eq!(bytes2.len(), 9);
        assert!(writer.is_empty());
    }
}
