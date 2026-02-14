use indexmap::IndexMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::BuildHasherDefault;

pub type Amf0IndexMap = IndexMap<String, Amf0ValueType, BuildHasherDefault<DefaultHasher>>;

#[derive(PartialEq, Clone, Debug)]
pub enum Amf0ValueType {
    Number(f64),
    Boolean(bool),
    UTF8String(String),
    Object(Amf0IndexMap),
    Null,
    EcmaArray(Amf0IndexMap),
    LongUTF8String(String),
    END,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amf0_value_type_number() {
        let val = Amf0ValueType::Number(42.0);
        assert_eq!(val, Amf0ValueType::Number(42.0));
        assert_ne!(val, Amf0ValueType::Number(43.0));
    }

    #[test]
    fn test_amf0_value_type_boolean() {
        let val = Amf0ValueType::Boolean(true);
        assert_eq!(val, Amf0ValueType::Boolean(true));
        assert_ne!(val, Amf0ValueType::Boolean(false));
    }

    #[test]
    fn test_amf0_value_type_utf8_string() {
        let val = Amf0ValueType::UTF8String("hello".to_string());
        assert_eq!(val, Amf0ValueType::UTF8String("hello".to_string()));
        assert_ne!(val, Amf0ValueType::UTF8String("world".to_string()));
    }

    #[test]
    fn test_amf0_value_type_null() {
        let val = Amf0ValueType::Null;
        assert_eq!(val, Amf0ValueType::Null);
    }

    #[test]
    fn test_amf0_value_type_end() {
        let val = Amf0ValueType::END;
        assert_eq!(val, Amf0ValueType::END);
    }

    #[test]
    fn test_amf0_value_type_long_utf8_string() {
        let val = Amf0ValueType::LongUTF8String("long text".to_string());
        assert_eq!(val, Amf0ValueType::LongUTF8String("long text".to_string()));
    }

    #[test]
    fn test_amf0_value_type_clone() {
        let val = Amf0ValueType::Number(3.14);
        let cloned = val.clone();
        assert_eq!(val, cloned);
    }

    #[test]
    fn test_amf0_value_type_debug() {
        let val = Amf0ValueType::Number(1.0);
        let debug = format!("{:?}", val);
        assert!(debug.contains("Number"));
    }

    #[test]
    fn test_amf0_value_type_object() {
        let mut map = Amf0IndexMap::default();
        map.insert("key".to_string(), Amf0ValueType::Number(1.0));
        let val = Amf0ValueType::Object(map.clone());
        assert_eq!(val, Amf0ValueType::Object(map));
    }

    #[test]
    fn test_amf0_value_type_ecma_array() {
        let mut map = Amf0IndexMap::default();
        map.insert("duration".to_string(), Amf0ValueType::Number(0.0));
        let val = Amf0ValueType::EcmaArray(map.clone());
        assert_eq!(val, Amf0ValueType::EcmaArray(map));
    }

    #[test]
    fn test_amf0_value_types_are_distinct() {
        let number = Amf0ValueType::Number(0.0);
        let boolean = Amf0ValueType::Boolean(false);
        let string = Amf0ValueType::UTF8String(String::new());
        let null = Amf0ValueType::Null;
        let end = Amf0ValueType::END;

        assert_ne!(number, boolean);
        assert_ne!(number, string);
        assert_ne!(number, null);
        assert_ne!(number, end);
        assert_ne!(boolean, null);
    }
}
