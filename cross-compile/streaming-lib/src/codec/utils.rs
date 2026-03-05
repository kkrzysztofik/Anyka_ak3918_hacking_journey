use super::errors::H264Error;
use crate::io::bits_reader::BitsReader;

// ue(v) in 9.1 Parsing process for Exp-Golomb codes
// ISO_IEC_14496-10-AVC-2012.pdf, page 227.
// Syntax elements coded as ue(v), me(v), or se(v) are Exp-Golomb-coded.
//      leadingZeroBits = -1;
//      for( b = 0; !b; leadingZeroBits++ )
//          b = read_bits( 1 )
// The variable codeNum is then assigned as follows:
//      codeNum = (1 << leadingZeroBits) - 1 + read_bits( leadingZeroBits )
pub fn read_uev(bit_reader: &mut BitsReader) -> Result<u32, H264Error> {
    let mut leading_zeros_bits: usize = 0;

    loop {
        if bit_reader.read_bit()? != 0 {
            break;
        }
        leading_zeros_bits += 1;
    }
    let code_num = (1 << leading_zeros_bits) - 1 + bit_reader.read_n_bits(leading_zeros_bits)?;
    Ok(code_num as u32)
}

// ISO_IEC_14496-10-AVC-2012.pdf, page 229.
pub fn read_sev(bit_reader: &mut BitsReader) -> Result<i32, H264Error> {
    let code_num = read_uev(bit_reader)?;

    let negative: i64 = if code_num % 2 == 0 { -1 } else { 1 };
    let se_value = (code_num as i64 + 1) / 2 * negative;
    Ok(se_value as i32)
}

#[cfg(test)]
mod tests {

    use super::{read_sev, read_uev};
    use crate::io::bits_reader::BitsReader;
    use crate::io::bytes_reader::BytesReader;
    use bytes::BytesMut;

    #[test]
    fn test_read_uev() {
        // 0 => 1 => 1
        // 1 => 10 => 010
        // 2 => 11 => 011
        // 3 => 100 => 00100
        // 4 => 101 => 00101
        // 5 => 110 => 00110
        // 6 => 111 => 00111
        // 7 => 1000 => 0001000
        // 8 => 1001 => 0001001

        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0b00000001]);
        bytes_reader.extend_from_slice(&[0b00000010]);
        bytes_reader.extend_from_slice(&[0b00000011]);
        bytes_reader.extend_from_slice(&[0b00000100]);
        bytes_reader.extend_from_slice(&[0b00000101]);
        bytes_reader.extend_from_slice(&[0b00000110]);
        bytes_reader.extend_from_slice(&[0b00000111]);
        bytes_reader.extend_from_slice(&[0b00001000]);
        bytes_reader.extend_from_slice(&[0b00001001]);

        let mut bits_reader = BitsReader::new(bytes_reader);

        bits_reader.read_n_bits(7).unwrap();
        let v1 = read_uev(&mut bits_reader).unwrap();
        println!("=={v1}==");
        assert!(v1 == 0);

        bits_reader.read_n_bits(5).unwrap();
        let v2 = read_uev(&mut bits_reader).unwrap();
        println!("=={v2}==");
        assert!(v2 == 1);

        bits_reader.read_n_bits(5).unwrap();
        let v3 = read_uev(&mut bits_reader).unwrap();
        println!("=={v3}==");
        assert!(v3 == 2);

        bits_reader.read_n_bits(3).unwrap();
        let v4 = read_uev(&mut bits_reader).unwrap();
        println!("=={v4}==");
        assert!(v4 == 3);

        bits_reader.read_n_bits(3).unwrap();
        let v5 = read_uev(&mut bits_reader).unwrap();
        println!("=={v5}==");
        assert!(v5 == 4);

        bits_reader.read_n_bits(3).unwrap();
        let v6 = read_uev(&mut bits_reader).unwrap();
        println!("=={v6}==");
        assert!(v6 == 5);

        bits_reader.read_n_bits(3).unwrap();
        let v7 = read_uev(&mut bits_reader).unwrap();
        println!("=={v7}==");
        assert!(v7 == 6);

        bits_reader.read_n_bits(1).unwrap();
        let v8 = read_uev(&mut bits_reader).unwrap();
        println!("=={v8}==");
        assert!(v8 == 7);

        bits_reader.read_n_bits(1).unwrap();
        let v9 = read_uev(&mut bits_reader).unwrap();
        println!("=={v9}==");
        assert!(v9 == 8);
    }

    #[test]
    fn test_read_sev() {
        // code_num 0 -> "1" -> se(v) = 0
        // code_num 1 -> "010" -> se(v) = 1
        // code_num 2 -> "011" -> se(v) = -1
        // bitstream: 1 010 011 0 (pad)
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0b1010_0110]);

        let mut bits_reader = BitsReader::new(bytes_reader);

        let v1 = read_sev(&mut bits_reader).unwrap();
        let v2 = read_sev(&mut bits_reader).unwrap();
        let v3 = read_sev(&mut bits_reader).unwrap();

        assert_eq!(v1, 0);
        assert_eq!(v2, 1);
        assert_eq!(v3, -1);
    }

    #[test]
    fn test_read_sev_larger_values() {
        // code_num 3 -> "00100" -> se(v) = 2
        // code_num 4 -> "00101" -> se(v) = -2
        // code_num 5 -> "00110" -> se(v) = 3
        // code_num 6 -> "00111" -> se(v) = -3
        // bitstream: 00100 00101 00110 0 (pad)
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0b00100_001]);
        bytes_reader.extend_from_slice(&[0b01_00110_0]);

        let mut bits_reader = BitsReader::new(bytes_reader);

        let v1 = read_sev(&mut bits_reader).unwrap();
        let v2 = read_sev(&mut bits_reader).unwrap();
        let v3 = read_sev(&mut bits_reader).unwrap();

        assert_eq!(v1, 2);
        assert_eq!(v2, -2);
        assert_eq!(v3, 3);
    }

    #[test]
    fn test_read_uev_zero_value() {
        // uev(0) is encoded as a single "1" bit
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0b1000_0000]);

        let mut bits_reader = BitsReader::new(bytes_reader);
        let v = read_uev(&mut bits_reader).unwrap();
        assert_eq!(v, 0);
    }

    #[test]
    fn test_read_uev_value_one() {
        // uev(1) is encoded as "010"
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0b0100_0000]);

        let mut bits_reader = BitsReader::new(bytes_reader);
        let v = read_uev(&mut bits_reader).unwrap();
        assert_eq!(v, 1);
    }

    #[test]
    fn test_read_uev_value_two() {
        // uev(2) is encoded as "011"
        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&[0b0110_0000]);

        let mut bits_reader = BitsReader::new(bytes_reader);
        let v = read_uev(&mut bits_reader).unwrap();
        assert_eq!(v, 2);
    }
}
