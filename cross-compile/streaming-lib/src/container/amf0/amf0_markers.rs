pub const NUMBER: u8 = 0x00;
pub const BOOLEAN: u8 = 0x01;
pub const STRING: u8 = 0x02;
pub const OBJECT: u8 = 0x03;
pub const NULL: u8 = 0x05;
pub const ECMA_ARRAY: u8 = 0x08;
pub const OBJECT_END: u8 = 0x09;
pub const LONG_STRING: u8 = 0x0c;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amf0_marker_values() {
        assert_eq!(NUMBER, 0x00);
        assert_eq!(BOOLEAN, 0x01);
        assert_eq!(STRING, 0x02);
        assert_eq!(OBJECT, 0x03);
        assert_eq!(NULL, 0x05);
        assert_eq!(ECMA_ARRAY, 0x08);
        assert_eq!(OBJECT_END, 0x09);
        assert_eq!(LONG_STRING, 0x0c);
    }

    #[test]
    fn test_amf0_markers_are_distinct() {
        let markers = [
            NUMBER,
            BOOLEAN,
            STRING,
            OBJECT,
            NULL,
            ECMA_ARRAY,
            OBJECT_END,
            LONG_STRING,
        ];
        for i in 0..markers.len() {
            for j in (i + 1)..markers.len() {
                assert_ne!(markers[i], markers[j], "markers[{}] == markers[{}]", i, j);
            }
        }
    }

    #[test]
    fn test_amf0_marker_ordering() {
        // Verify known ordering constraints from AMF0 spec
        assert!(NUMBER < BOOLEAN);
        assert!(BOOLEAN < STRING);
        assert!(STRING < OBJECT);
        assert!(OBJECT < NULL);
        assert!(NULL < ECMA_ARRAY);
        assert!(ECMA_ARRAY < OBJECT_END);
        assert!(OBJECT_END < LONG_STRING);
    }
}
