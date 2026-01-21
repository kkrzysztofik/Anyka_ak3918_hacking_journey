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
