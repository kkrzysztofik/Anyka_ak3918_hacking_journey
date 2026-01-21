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
    log::debug!("{line}");
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
}
