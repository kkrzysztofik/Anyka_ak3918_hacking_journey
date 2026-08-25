//! Minimal BER helpers for SNMPv2c (not a full ASN.1 stack).

use thiserror::Error;

pub const TAG_INTEGER: u8 = 0x02;
pub const TAG_OCTET_STRING: u8 = 0x04;
pub const TAG_NULL: u8 = 0x05;
pub const TAG_OID: u8 = 0x06;
pub const TAG_SEQUENCE: u8 = 0x30;

/// Object identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Oid(pub Vec<u32>);

impl Oid {
    /// Build an OID from arcs. Requires at least two arcs with first in 0..=2.
    pub fn from_slice(arcs: &[u32]) -> Result<Self, BerError> {
        if arcs.len() < 2 || arcs[0] > 2 {
            return Err(BerError::InvalidOid);
        }
        if arcs[0] < 2 && arcs[1] >= 40 {
            return Err(BerError::InvalidOid);
        }
        Ok(Self(arcs.to_vec()))
    }

    /// Encode OID content bytes (no tag/length).
    pub fn encode(&self) -> Result<Vec<u8>, BerError> {
        if self.0.len() < 2 {
            return Err(BerError::InvalidOid);
        }
        let mut out = Vec::new();
        let first = self.0[0]
            .checked_mul(40)
            .and_then(|v| v.checked_add(self.0[1]))
            .ok_or(BerError::InvalidOid)?;
        if first > u8::MAX as u32 {
            return Err(BerError::InvalidOid);
        }
        out.push(first as u8);
        for &arc in &self.0[2..] {
            encode_base128(arc, &mut out);
        }
        Ok(out)
    }

    /// Decode OID content bytes (no tag/length).
    pub fn decode(bytes: &[u8]) -> Result<Self, BerError> {
        if bytes.is_empty() {
            return Err(BerError::InvalidOid);
        }
        let first = bytes[0] as u32;
        let mut arcs = vec![first / 40, first % 40];
        let mut i = 1;
        while i < bytes.len() {
            let (arc, next) = decode_base128(bytes, i)?;
            arcs.push(arc);
            i = next;
        }
        Self::from_slice(&arcs)
    }
}

fn encode_base128(mut value: u32, out: &mut Vec<u8>) {
    let mut stack = [0u8; 5];
    let mut n = 0;
    loop {
        stack[n] = (value & 0x7f) as u8;
        n += 1;
        value >>= 7;
        if value == 0 {
            break;
        }
    }
    while n > 1 {
        n -= 1;
        out.push(stack[n] | 0x80);
    }
    out.push(stack[0]);
}

fn decode_base128(bytes: &[u8], mut i: usize) -> Result<(u32, usize), BerError> {
    let mut value: u32 = 0;
    loop {
        if i >= bytes.len() {
            return Err(BerError::Truncated);
        }
        let b = bytes[i];
        i += 1;
        value = value
            .checked_shl(7)
            .and_then(|v| v.checked_add(u32::from(b & 0x7f)))
            .ok_or(BerError::InvalidOid)?;
        if b & 0x80 == 0 {
            return Ok((value, i));
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BerError {
    #[error("invalid OID")]
    InvalidOid,
    #[error("truncated BER")]
    Truncated,
    #[error("unexpected tag")]
    UnexpectedTag,
    #[error("unsupported BER")]
    Unsupported,
}

/// Read tag, length, and content slice; returns (tag, content, rest).
pub fn read_tlv(input: &[u8]) -> Result<(u8, &[u8], &[u8]), BerError> {
    if input.len() < 2 {
        return Err(BerError::Truncated);
    }
    let tag = input[0];
    let (len, after_len) = read_length(&input[1..])?;
    if after_len.len() < len {
        return Err(BerError::Truncated);
    }
    let (content, rest) = after_len.split_at(len);
    Ok((tag, content, rest))
}

fn read_length(input: &[u8]) -> Result<(usize, &[u8]), BerError> {
    if input.is_empty() {
        return Err(BerError::Truncated);
    }
    let first = input[0];
    if first & 0x80 == 0 {
        return Ok((first as usize, &input[1..]));
    }
    let nbytes = (first & 0x7f) as usize;
    if nbytes == 0 || nbytes > 4 || input.len() < 1 + nbytes {
        return Err(BerError::Unsupported);
    }
    let mut len = 0usize;
    for &b in &input[1..1 + nbytes] {
        len = (len << 8) | b as usize;
    }
    Ok((len, &input[1 + nbytes..]))
}

pub fn write_tlv(tag: u8, content: &[u8], out: &mut Vec<u8>) {
    out.push(tag);
    write_length(content.len(), out);
    out.extend_from_slice(content);
}

fn write_length(len: usize, out: &mut Vec<u8>) {
    if len < 0x80 {
        out.push(len as u8);
        return;
    }
    // Definite long form — enough for SNMP PDUs we emit.
    let bytes = len.to_be_bytes();
    let start = bytes
        .iter()
        .position(|&b| b != 0)
        .unwrap_or(bytes.len() - 1);
    let significant = &bytes[start..];
    out.push(0x80 | significant.len() as u8);
    out.extend_from_slice(significant);
}

pub fn decode_integer(content: &[u8]) -> Result<i32, BerError> {
    if content.is_empty() || content.len() > 4 {
        return Err(BerError::Unsupported);
    }
    let mut value: i32 = if content[0] & 0x80 != 0 { -1 } else { 0 };
    for &b in content {
        value = (value << 8) | i32::from(b);
    }
    Ok(value)
}

pub fn encode_integer(value: i32) -> Vec<u8> {
    let mut bytes = value.to_be_bytes().to_vec();
    // Minimal two's-complement encoding.
    while bytes.len() > 1
        && ((bytes[0] == 0x00 && bytes[1] & 0x80 == 0)
            || (bytes[0] == 0xff && bytes[1] & 0x80 != 0))
    {
        bytes.remove(0);
    }
    bytes
}

/// Encode a u32 for the unsigned application types (Counter32/Gauge32/TimeTicks).
///
/// BER integers are two's complement, so a value with the top bit set needs a
/// leading zero to stay non-negative — this is what net-snmp emits. Routing
/// these through `encode_integer` instead strips leading `0xff` bytes and turns
/// `0xFFFF_FFFF` into `255` on the wire.
pub fn encode_unsigned(value: u32) -> Vec<u8> {
    let mut bytes = value.to_be_bytes().to_vec();
    while bytes.len() > 1 && bytes[0] == 0 {
        bytes.remove(0);
    }
    if bytes[0] & 0x80 != 0 {
        bytes.insert(0, 0);
    }
    bytes
}

pub fn expect_tag(input: &[u8], tag: u8) -> Result<(&[u8], &[u8]), BerError> {
    let (got, content, rest) = read_tlv(input)?;
    if got != tag {
        return Err(BerError::UnexpectedTag);
    }
    Ok((content, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oid_sysdescr_round_trip() {
        let oid = Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 1, 0]).expect("oid");
        let encoded = oid.encode().expect("encode");
        assert_eq!(
            encoded,
            vec![0x2b, 0x06, 0x01, 0x02, 0x01, 0x01, 0x01, 0x00]
        );
        let decoded = Oid::decode(&encoded).expect("decode");
        assert_eq!(decoded, oid);
    }

    #[test]
    fn test_oid_from_slice_rejects_short_and_invalid_first_arc() {
        assert_eq!(Oid::from_slice(&[1]), Err(BerError::InvalidOid));
        assert_eq!(Oid::from_slice(&[3, 1]), Err(BerError::InvalidOid));
        assert_eq!(Oid::from_slice(&[1, 40]), Err(BerError::InvalidOid));
    }

    #[test]
    fn test_oid_encode_rejects_empty_and_overflow_first_byte() {
        assert_eq!(Oid(vec![]).encode(), Err(BerError::InvalidOid));
        assert_eq!(Oid(vec![2, 200]).encode(), Err(BerError::InvalidOid));
    }

    #[test]
    fn test_oid_decode_empty_and_truncated_base128() {
        assert_eq!(Oid::decode(&[]), Err(BerError::InvalidOid));
        assert_eq!(Oid::decode(&[0x2b, 0x81]), Err(BerError::Truncated));
    }

    #[test]
    fn test_oid_round_trip_with_large_arc() {
        let oid = Oid::from_slice(&[1, 3, 6, 1, 4, 1, 99999]).unwrap();
        let encoded = oid.encode().unwrap();
        assert!(encoded.len() > 5);
        assert_eq!(Oid::decode(&encoded).unwrap(), oid);
    }

    #[test]
    fn test_read_tlv_truncated_and_long_length() {
        assert_eq!(read_tlv(&[0x04]), Err(BerError::Truncated));
        assert_eq!(read_tlv(&[0x04, 0x02, 0x00]), Err(BerError::Truncated));
        // Long-form length: 0x81 0x01 means length=1
        let (tag, content, rest) = read_tlv(&[0x04, 0x81, 0x01, b'x', 0xff]).unwrap();
        assert_eq!(tag, TAG_OCTET_STRING);
        assert_eq!(content, b"x");
        assert_eq!(rest, &[0xff]);
        assert_eq!(read_length(&[]), Err(BerError::Truncated));
        assert_eq!(read_length(&[0x80]), Err(BerError::Unsupported));
        assert_eq!(read_length(&[0x85, 0, 0, 0, 0]), Err(BerError::Unsupported));
    }

    #[test]
    fn test_write_tlv_long_length_form() {
        let content = vec![0u8; 200];
        let mut out = Vec::new();
        write_tlv(TAG_OCTET_STRING, &content, &mut out);
        assert_eq!(out[0], TAG_OCTET_STRING);
        assert_eq!(out[1], 0x81);
        assert_eq!(out[2], 200);
        assert_eq!(&out[3..], content.as_slice());
    }

    #[test]
    fn test_decode_encode_integer_edge_cases() {
        assert_eq!(decode_integer(&[]), Err(BerError::Unsupported));
        assert_eq!(decode_integer(&[0, 0, 0, 0, 1]), Err(BerError::Unsupported));
        assert_eq!(decode_integer(&[0xff]).unwrap(), -1);
        assert_eq!(encode_integer(-1), vec![0xff]);
        assert_eq!(encode_integer(128), vec![0x00, 0x80]);
        let (content, rest) = expect_tag(&[0x02, 0x01, 0x07, 0xaa], TAG_INTEGER).unwrap();
        assert_eq!(decode_integer(content).unwrap(), 7);
        assert_eq!(rest, &[0xaa]);
        assert_eq!(
            expect_tag(&[0x04, 0x00], TAG_INTEGER),
            Err(BerError::UnexpectedTag)
        );
    }

#[test]
    fn test_encode_unsigned_never_strips_ff() {
        assert_eq!(encode_unsigned(0), vec![0x00]);
        assert_eq!(encode_unsigned(200), vec![0x00, 0xc8]);
        assert_eq!(encode_unsigned(0x7fff_ffff), vec![0x7f, 0xff, 0xff, 0xff]);
        // The bug: encode_integer(-1) would give [0xff] and read back as 255.
        assert_eq!(encode_unsigned(u32::MAX), vec![0x00, 0xff, 0xff, 0xff, 0xff]);
        assert_eq!(encode_unsigned(0xffff_ff00), vec![0x00, 0xff, 0xff, 0xff, 0x00]);
    }
}
