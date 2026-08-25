//! SNMPv2c message and PDU encode/decode.

use crate::ber::{
    self, BerError, Oid, TAG_INTEGER, TAG_NULL, TAG_OCTET_STRING, TAG_OID, TAG_SEQUENCE,
};
use thiserror::Error;

/// SNMPv2c wire version (INTEGER 1).
pub const SNMP_V2C_VERSION: i32 = 1;

const PDU_GET_REQUEST: u8 = 0xa0;
const PDU_GET_NEXT_REQUEST: u8 = 0xa1;
const PDU_GET_RESPONSE: u8 = 0xa2;
const PDU_SET_REQUEST: u8 = 0xa3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PduType {
    GetRequest,
    GetNextRequest,
    GetResponse,
    SetRequest,
}

impl PduType {
    fn tag(self) -> u8 {
        match self {
            Self::GetRequest => PDU_GET_REQUEST,
            Self::GetNextRequest => PDU_GET_NEXT_REQUEST,
            Self::GetResponse => PDU_GET_RESPONSE,
            Self::SetRequest => PDU_SET_REQUEST,
        }
    }

    fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            PDU_GET_REQUEST => Some(Self::GetRequest),
            PDU_GET_NEXT_REQUEST => Some(Self::GetNextRequest),
            PDU_GET_RESPONSE => Some(Self::GetResponse),
            PDU_SET_REQUEST => Some(Self::SetRequest),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarBind {
    pub name: Oid,
    pub value: SnmpValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnmpValue {
    Null,
    Integer(i32),
    OctetString(Vec<u8>),
    ObjectId(Oid),
    /// TimeTicks (application tag 3) — hundredths of a second.
    TimeTicks(u32),
    /// Counter32 (application tag 1).
    Counter32(u32),
    /// Gauge32 (application tag 2).
    Gauge32(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pdu {
    pub pdu_type: PduType,
    pub request_id: i32,
    pub error_status: i32,
    pub error_index: i32,
    pub variable_bindings: Vec<VarBind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnmpMessage {
    pub version: i32,
    pub community: String,
    pub pdu: Pdu,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PduError {
    #[error(transparent)]
    Ber(#[from] BerError),
    #[error("unsupported SNMP version {0}")]
    UnsupportedVersion(i32),
    #[error("malformed SNMP message")]
    Malformed,
}

impl SnmpMessage {
    pub fn parse(bytes: &[u8]) -> Result<Self, PduError> {
        let (seq, rest) = ber::expect_tag(bytes, TAG_SEQUENCE)?;
        if !rest.is_empty() {
            return Err(PduError::Malformed);
        }

        let (ver_content, rest) = ber::expect_tag(seq, TAG_INTEGER)?;
        let version = ber::decode_integer(ver_content)?;
        if version != SNMP_V2C_VERSION {
            return Err(PduError::UnsupportedVersion(version));
        }

        let (community_bytes, rest) = ber::expect_tag(rest, TAG_OCTET_STRING)?;
        let community = std::str::from_utf8(community_bytes)
            .map_err(|_| PduError::Malformed)?
            .to_string();

        let (pdu_tag, pdu_content, rest) = ber::read_tlv(rest)?;
        if !rest.is_empty() {
            return Err(PduError::Malformed);
        }
        let pdu_type = PduType::from_tag(pdu_tag).ok_or(PduError::Malformed)?;
        let pdu = parse_pdu_body(pdu_type, pdu_content)?;

        Ok(Self {
            version,
            community,
            pdu,
        })
    }

    /// Encode a response (or any PDU) as an SNMPv2c message.
    pub fn encode(&self) -> Result<Vec<u8>, PduError> {
        let mut inner = Vec::new();
        ber::write_tlv(TAG_INTEGER, &ber::encode_integer(self.version), &mut inner);
        ber::write_tlv(TAG_OCTET_STRING, self.community.as_bytes(), &mut inner);
        let pdu_bytes = encode_pdu(&self.pdu)?;
        inner.extend_from_slice(&pdu_bytes);

        let mut out = Vec::new();
        ber::write_tlv(TAG_SEQUENCE, &inner, &mut out);
        Ok(out)
    }
}

fn parse_pdu_body(pdu_type: PduType, content: &[u8]) -> Result<Pdu, PduError> {
    let (id_c, rest) = ber::expect_tag(content, TAG_INTEGER)?;
    let request_id = ber::decode_integer(id_c)?;
    let (es_c, rest) = ber::expect_tag(rest, TAG_INTEGER)?;
    let error_status = ber::decode_integer(es_c)?;
    let (ei_c, rest) = ber::expect_tag(rest, TAG_INTEGER)?;
    let error_index = ber::decode_integer(ei_c)?;
    let (vbl_c, rest) = ber::expect_tag(rest, TAG_SEQUENCE)?;
    if !rest.is_empty() {
        return Err(PduError::Malformed);
    }
    let variable_bindings = parse_varbind_list(vbl_c)?;
    Ok(Pdu {
        pdu_type,
        request_id,
        error_status,
        error_index,
        variable_bindings,
    })
}

fn parse_varbind_list(mut input: &[u8]) -> Result<Vec<VarBind>, PduError> {
    let mut out = Vec::new();
    while !input.is_empty() {
        let (vb, rest) = ber::expect_tag(input, TAG_SEQUENCE)?;
        input = rest;
        let (oid_c, rest) = ber::expect_tag(vb, TAG_OID)?;
        let name = Oid::decode(oid_c)?;
        let (val_tag, val_c, rest) = ber::read_tlv(rest)?;
        if !rest.is_empty() {
            return Err(PduError::Malformed);
        }
        let value = decode_value(val_tag, val_c)?;
        out.push(VarBind { name, value });
    }
    Ok(out)
}

const TAG_COUNTER32: u8 = 0x41; // Application 1
const TAG_GAUGE32: u8 = 0x42; // Application 2
const TAG_TIMETICKS: u8 = 0x43; // Application 3

fn decode_u32_app(content: &[u8]) -> Result<u32, PduError> {
    // Up to 5 bytes: real agents pad values with the top bit set with a leading zero.
    if content.is_empty() || content.len() > 5 {
        return Err(PduError::Malformed);
    }
    if content.len() == 5 && content[0] != 0 {
        return Err(PduError::Malformed);
    }
    let mut value: u64 = 0;
    for &b in content {
        value = (value << 8) | u64::from(b);
    }
    u32::try_from(value).map_err(|_| PduError::Malformed)
}

fn decode_value(tag: u8, content: &[u8]) -> Result<SnmpValue, PduError> {
    match tag {
        TAG_NULL if content.is_empty() => Ok(SnmpValue::Null),
        TAG_INTEGER => Ok(SnmpValue::Integer(ber::decode_integer(content)?)),
        TAG_OCTET_STRING => Ok(SnmpValue::OctetString(content.to_vec())),
        TAG_OID => Ok(SnmpValue::ObjectId(Oid::decode(content)?)),
        TAG_COUNTER32 => Ok(SnmpValue::Counter32(decode_u32_app(content)?)),
        TAG_GAUGE32 => Ok(SnmpValue::Gauge32(decode_u32_app(content)?)),
        TAG_TIMETICKS => Ok(SnmpValue::TimeTicks(decode_u32_app(content)?)),
        _ => Err(PduError::Malformed),
    }
}

fn encode_value(value: &SnmpValue, out: &mut Vec<u8>) -> Result<(), PduError> {
    match value {
        SnmpValue::Null => ber::write_tlv(TAG_NULL, &[], out),
        SnmpValue::Integer(v) => ber::write_tlv(TAG_INTEGER, &ber::encode_integer(*v), out),
        SnmpValue::OctetString(b) => ber::write_tlv(TAG_OCTET_STRING, b, out),
        SnmpValue::ObjectId(oid) => ber::write_tlv(TAG_OID, &oid.encode()?, out),
        SnmpValue::Counter32(v) => ber::write_tlv(TAG_COUNTER32, &ber::encode_unsigned(*v), out),
        SnmpValue::Gauge32(v) => ber::write_tlv(TAG_GAUGE32, &ber::encode_unsigned(*v), out),
        SnmpValue::TimeTicks(t) => ber::write_tlv(TAG_TIMETICKS, &ber::encode_unsigned(*t), out),
    }
    Ok(())
}

fn encode_pdu(pdu: &Pdu) -> Result<Vec<u8>, PduError> {
    let mut body = Vec::new();
    ber::write_tlv(TAG_INTEGER, &ber::encode_integer(pdu.request_id), &mut body);
    ber::write_tlv(
        TAG_INTEGER,
        &ber::encode_integer(pdu.error_status),
        &mut body,
    );
    ber::write_tlv(
        TAG_INTEGER,
        &ber::encode_integer(pdu.error_index),
        &mut body,
    );

    let mut vbl = Vec::new();
    for vb in &pdu.variable_bindings {
        let mut vb_bytes = Vec::new();
        let oid_content = vb.name.encode()?;
        ber::write_tlv(TAG_OID, &oid_content, &mut vb_bytes);
        encode_value(&vb.value, &mut vb_bytes)?;
        ber::write_tlv(TAG_SEQUENCE, &vb_bytes, &mut vbl);
    }
    ber::write_tlv(TAG_SEQUENCE, &vbl, &mut body);

    let mut out = Vec::new();
    ber::write_tlv(pdu.pdu_type.tag(), &body, &mut out);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-built SNMPv2c GetRequest for sysDescr.0, community "public".
    fn hand_built_get_sysdescr() -> Vec<u8> {
        vec![
            0x30, 0x26, // SEQUENCE len 38
            0x02, 0x01, 0x01, // version 1
            0x04, 0x06, b'p', b'u', b'b', b'l', b'i', b'c', 0xa0,
            0x19, // GetRequest [0] len 25
            0x02, 0x01, 0x01, // request-id
            0x02, 0x01, 0x00, // error-status
            0x02, 0x01, 0x00, // error-index
            0x30, 0x0e, // VarBindList len 14
            0x30, 0x0c, // VarBind len 12
            0x06, 0x08, 0x2b, 0x06, 0x01, 0x02, 0x01, 0x01, 0x01, 0x00, 0x05, 0x00, // NULL
        ]
    }

    #[test]
    fn test_parse_get_sysdescr_public() {
        let msg = SnmpMessage::parse(&hand_built_get_sysdescr()).expect("parse");
        assert_eq!(msg.version, SNMP_V2C_VERSION);
        assert_eq!(msg.community, "public");
        assert_eq!(msg.pdu.pdu_type, PduType::GetRequest);
        assert_eq!(msg.pdu.request_id, 1);
        assert_eq!(msg.pdu.error_status, 0);
        assert_eq!(msg.pdu.variable_bindings.len(), 1);
        assert_eq!(
            msg.pdu.variable_bindings[0].name,
            Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 1, 0]).unwrap()
        );
        assert_eq!(msg.pdu.variable_bindings[0].value, SnmpValue::Null);
    }

    #[test]
    fn test_reject_non_v2c_version() {
        let mut bytes = hand_built_get_sysdescr();
        bytes[4] = 0; // SNMPv1
        let err = SnmpMessage::parse(&bytes).expect_err("must reject v1");
        assert!(matches!(err, PduError::UnsupportedVersion(0)));
    }

    #[test]
    fn test_encode_round_trips_parsed_get() {
        let msg = SnmpMessage::parse(&hand_built_get_sysdescr()).expect("parse");
        let encoded = msg.encode().expect("encode");
        let again = SnmpMessage::parse(&encoded).expect("re-parse");
        assert_eq!(again, msg);
    }

    #[test]
    fn test_encode_round_trips_all_value_types() {
        let oid = Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 2, 0]).unwrap();
        let msg = SnmpMessage {
            version: SNMP_V2C_VERSION,
            community: "public".into(),
            pdu: Pdu {
                pdu_type: PduType::GetResponse,
                request_id: 9,
                error_status: 0,
                error_index: 0,
                variable_bindings: vec![
                    VarBind {
                        name: oid.clone(),
                        value: SnmpValue::ObjectId(oid.clone()),
                    },
                    VarBind {
                        name: oid.clone(),
                        value: SnmpValue::Integer(-5),
                    },
                    VarBind {
                        name: oid.clone(),
                        value: SnmpValue::OctetString(b"x".to_vec()),
                    },
                    VarBind {
                        name: oid.clone(),
                        value: SnmpValue::Counter32(42),
                    },
                    VarBind {
                        name: oid.clone(),
                        value: SnmpValue::Gauge32(7),
                    },
                    VarBind {
                        name: oid.clone(),
                        value: SnmpValue::TimeTicks(100),
                    },
                ],
            },
        };
        let encoded = msg.encode().unwrap();
        let again = SnmpMessage::parse(&encoded).unwrap();
        assert_eq!(again, msg);
    }

    #[test]
    fn test_pdu_type_tags_cover_getnext_and_set() {
        assert_eq!(PduType::GetNextRequest.tag(), 0xa1);
        assert_eq!(PduType::SetRequest.tag(), 0xa3);
        assert_eq!(PduType::from_tag(0xa1), Some(PduType::GetNextRequest));
        assert_eq!(PduType::from_tag(0xa3), Some(PduType::SetRequest));
        assert_eq!(PduType::from_tag(0x99), None);
    }

    #[test]
    fn test_parse_rejects_trailing_bytes_and_bad_community_utf8() {
        let mut bytes = hand_built_get_sysdescr();
        bytes.push(0x00);
        assert!(matches!(
            SnmpMessage::parse(&bytes),
            Err(PduError::Malformed)
        ));

        let mut bad = hand_built_get_sysdescr();
        // community bytes start at index 7 for "public"
        bad[7] = 0xff;
        assert!(matches!(SnmpMessage::parse(&bad), Err(PduError::Malformed)));
    }

    #[test]
    fn test_decode_value_rejects_unknown_tag_and_oversized_unsigned() {
        assert!(matches!(decode_value(0x99, &[]), Err(PduError::Malformed)));
        assert_eq!(decode_u32_app(&[0xff]).unwrap(), 255);
        assert!(matches!(
            decode_u32_app(&[0x01, 0, 0, 0, 0]),
            Err(PduError::Malformed)
        ));
        assert!(matches!(
            decode_u32_app(&[0, 0, 0, 0, 0, 0]),
            Err(PduError::Malformed)
        ));
    }

#[test]
    fn test_counter32_max_round_trips() {
        let oid = Oid::from_slice(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 10, 1]).unwrap();
        let msg = SnmpMessage {
            version: SNMP_V2C_VERSION,
            community: "public".into(),
            pdu: Pdu {
                pdu_type: PduType::GetResponse,
                request_id: 1,
                error_status: 0,
                error_index: 0,
                variable_bindings: vec![VarBind {
                    name: oid,
                    value: SnmpValue::Counter32(u32::MAX),
                }],
            },
        };
        let again = SnmpMessage::parse(&msg.encode().unwrap()).unwrap();
        assert_eq!(
            again.pdu.variable_bindings[0].value,
            SnmpValue::Counter32(u32::MAX)
        );
    }
}
