#[macro_export]
macro_rules! scanf {
    ( $string:expr, $sep:expr, $( $x:ty ),+ ) => {{
        let mut iter = $string.split($sep);
        ($(iter.next().and_then(|word| word.parse::<$x>().ok()),)*)
    }}
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_scanf_single_string() {
        let (val,) = scanf!("hello", " ", String);
        assert_eq!(val, Some("hello".to_string()));
    }

    #[test]
    fn test_scanf_two_integers() {
        let (a, b) = scanf!("10 20", " ", u32, u32);
        assert_eq!(a, Some(10));
        assert_eq!(b, Some(20));
    }

    #[test]
    fn test_scanf_mixed_types() {
        let (name, age) = scanf!("alice:30", ":", String, u32);
        assert_eq!(name, Some("alice".to_string()));
        assert_eq!(age, Some(30));
    }

    #[test]
    fn test_scanf_parse_failure_returns_none() {
        let (a, b) = scanf!("hello world", " ", u32, u32);
        assert_eq!(a, None);
        assert_eq!(b, None);
    }

    #[test]
    fn test_scanf_missing_fields_returns_none() {
        let (a, b, c) = scanf!("1 2", " ", u32, u32, u32);
        assert_eq!(a, Some(1));
        assert_eq!(b, Some(2));
        assert_eq!(c, None);
    }

    #[test]
    fn test_scanf_ip_address_parts() {
        let (a, b, c, d) = scanf!("192.168.1.1", ".", u8, u8, u8, u8);
        assert_eq!(a, Some(192));
        assert_eq!(b, Some(168));
        assert_eq!(c, Some(1));
        assert_eq!(d, Some(1));
    }

    #[test]
    fn test_scanf_empty_string() {
        let (a,) = scanf!("", " ", u32);
        assert_eq!(a, None);
    }
}
