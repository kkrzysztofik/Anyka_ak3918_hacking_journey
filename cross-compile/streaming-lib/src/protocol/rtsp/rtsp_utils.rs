macro_rules! scanf {
    ( $string:expr, $sep:expr, $( $x:ty ),+ ) => {{
        let mut iter = $string.split($sep);
        ($(iter.next().and_then(|word| word.parse::<$x>().ok()),)*)
    }}
}

pub(crate) use scanf;

pub fn debug_print_hex(title: &str, data: &[u8]) {
    let mut line = String::new();
    line.push_str(&format!("==========={}:{}\n", title, data.len()));

    for (idx, byte) in data.iter().enumerate() {
        line.push_str(&format!("{byte:02X} "));
        if (idx + 1) % 16 == 0 {
            line.push('\n');
        }
    }

    line.push_str("\n===========");
    tracing::debug!(%line);
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_scanf() {
        let str_a = "18:23:08";

        if let (Some(a), Some(b), Some(c), d) =
            scanf!(str_a, |c| c == ':' || c == '.', i64, i64, i64, i64)
        {
            assert_eq!(a, 18);
            assert_eq!(b, 23);
            assert_eq!(c, 8);
            assert!(d.is_none());
        } else {
            panic!("scanf did not parse expected values");
        }
    }

    #[test]
    fn test_scanf_with_dot_separator() {
        let str_a = "1.2.3";
        if let (Some(a), Some(b), Some(c)) = scanf!(str_a, |c| c == '.', u32, u32, u32) {
            assert_eq!(a, 1);
            assert_eq!(b, 2);
            assert_eq!(c, 3);
        } else {
            panic!("scanf did not parse dot-separated values");
        }
    }

    #[test]
    fn test_scanf_single_value() {
        let str_a = "42";
        if let (Some(a),) = scanf!(str_a, |c| c == ':', i64) {
            assert_eq!(a, 42);
        } else {
            panic!("scanf did not parse single value");
        }
    }

    #[test]
    fn test_scanf_empty_string() {
        let str_a = "";
        let (a,) = scanf!(str_a, |c| c == ':', i64);
        assert!(a.is_none());
    }

    #[test]
    fn test_scanf_non_numeric() {
        let str_a = "abc:def";
        let (a, b) = scanf!(str_a, |c| c == ':', i64, i64);
        assert!(a.is_none());
        assert!(b.is_none());
    }

    #[test]
    fn test_debug_print_hex_empty() {
        // Just verifying it doesn't panic with empty data
        super::debug_print_hex("test", &[]);
    }

    #[test]
    fn test_debug_print_hex_small_data() {
        // Verify no panic with small data
        super::debug_print_hex("test", &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_debug_print_hex_exactly_16_bytes() {
        // Test boundary at 16-byte line wrap
        let data: Vec<u8> = (0..16).collect();
        super::debug_print_hex("boundary", &data);
    }

    #[test]
    fn test_debug_print_hex_more_than_16_bytes() {
        // Test multiple lines
        let data: Vec<u8> = (0..33).collect();
        super::debug_print_hex("multiline", &data);
    }
}
