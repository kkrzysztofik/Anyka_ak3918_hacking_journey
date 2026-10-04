//! SNMPv2c message and PDU encode/decode.

use crate::ber::{
    self, Malformed, Oid, TAG_INTEGER, TAG_NULL, TAG_OCTET_STRING, TAG_OID, TAG_SEQUENCE,
};

/// SNMPv2c wire version (INTEGER 1).
const SNMP_V2C_VERSION: i32 = 1;

/// SNMP PDU type; the discriminant is the context-specific BER tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PduType {
    GetRequest = 0xa0,
    GetNextRequest = 0xa1,
    GetResponse = 0xa2,
    SetRequest = 0xa3,
    /// SNMPv2c GetBulkRequest — `error_status`/`error_index` hold non-repeaters / max-repetitions.
    GetBulkRequest = 0xa5,
}

impl PduType {
    fn from_tag(tag: u8) -> Option<Self> {
        [
            Self::GetRequest,
            Self::GetNextRequest,
            Self::GetResponse,
            Self::SetRequest,
            Self::GetBulkRequest,
        ]
        .into_iter()
        .find(|t| *t as u8 == tag)
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
    /// v2c exception: the object does not exist in this MIB view (context tag [0]).
    NoSuchObject,
    /// v2c exception: the object exists but this instance does not (context tag [1]).
    NoSuchInstance,
    /// v2c exception: no object follows this OID (context tag [2]).
    EndOfMibView,
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
    pub community: String,
    pub pdu: Pdu,
}

impl SnmpMessage {
    pub fn parse(bytes: &[u8]) -> Result<Self, Malformed> {
        let (seq, rest) = ber::expect_tag(bytes, TAG_SEQUENCE)?;
        if !rest.is_empty() {
            return Err(Malformed);
        }

        let (ver_content, rest) = ber::expect_tag(seq, TAG_INTEGER)?;
        let version = ber::decode_integer(ver_content)?;
        if version != SNMP_V2C_VERSION {
            return Err(Malformed);
        }

        let (community_bytes, rest) = ber::expect_tag(rest, TAG_OCTET_STRING)?;
        let community = std::str::from_utf8(community_bytes)
            .map_err(|_| Malformed)?
            .to_string();

        let (pdu_tag, pdu_content, rest) = ber::read_tlv(rest)?;
        if !rest.is_empty() {
            return Err(Malformed);
        }
        let pdu_type = PduType::from_tag(pdu_tag).ok_or(Malformed)?;
        let pdu = parse_pdu_body(pdu_type, pdu_content)?;

        Ok(Self { community, pdu })
    }

    /// Encode a response (or any PDU) as an SNMPv2c message.
    pub fn encode(&self) -> Result<Vec<u8>, Malformed> {
        let mut inner = Vec::new();
        ber::write_tlv(
            TAG_INTEGER,
            &ber::encode_integer(SNMP_V2C_VERSION),
            &mut inner,
        );
        ber::write_tlv(TAG_OCTET_STRING, self.community.as_bytes(), &mut inner);
        let pdu_bytes = encode_pdu(&self.pdu)?;
        inner.extend_from_slice(&pdu_bytes);

        let mut out = Vec::new();
        ber::write_tlv(TAG_SEQUENCE, &inner, &mut out);
        Ok(out)
    }
}

fn parse_pdu_body(pdu_type: PduType, content: &[u8]) -> Result<Pdu, Malformed> {
    let (id_c, rest) = ber::expect_tag(content, TAG_INTEGER)?;
    let request_id = ber::decode_integer(id_c)?;
    let (es_c, rest) = ber::expect_tag(rest, TAG_INTEGER)?;
    let error_status = ber::decode_integer(es_c)?;
    let (ei_c, rest) = ber::expect_tag(rest, TAG_INTEGER)?;
    let error_index = ber::decode_integer(ei_c)?;
    let (vbl_c, rest) = ber::expect_tag(rest, TAG_SEQUENCE)?;
    if !rest.is_empty() {
        return Err(Malformed);
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

fn parse_varbind_list(mut input: &[u8]) -> Result<Vec<VarBind>, Malformed> {
    let mut out = Vec::new();
    while !input.is_empty() {
        let (vb, rest) = ber::expect_tag(input, TAG_SEQUENCE)?;
        input = rest;
        let (oid_c, rest) = ber::expect_tag(vb, TAG_OID)?;
        let name = Oid::decode(oid_c)?;
        let (val_tag, val_c, rest) = ber::read_tlv(rest)?;
        if !rest.is_empty() {
            return Err(Malformed);
        }
        let value = decode_value(val_tag, val_c)?;
        out.push(VarBind { name, value });
    }
    Ok(out)
}

const TAG_COUNTER32: u8 = 0x41; // Application 1
const TAG_GAUGE32: u8 = 0x42; // Application 2
const TAG_TIMETICKS: u8 = 0x43; // Application 3
const TAG_NO_SUCH_OBJECT: u8 = 0x80;
const TAG_NO_SUCH_INSTANCE: u8 = 0x81;
const TAG_END_OF_MIB_VIEW: u8 = 0x82;

fn decode_u32_app(content: &[u8]) -> Result<u32, Malformed> {
    // Up to 5 bytes: real agents pad values with the top bit set with a leading zero.
    if content.is_empty() || content.len() > 5 {
        return Err(Malformed);
    }
    if content.len() == 5 && content[0] != 0 {
        return Err(Malformed);
    }
    let mut value: u64 = 0;
    for &b in content {
        value = (value << 8) | u64::from(b);
    }
    u32::try_from(value).map_err(|_| Malformed)
}

fn decode_value(tag: u8, content: &[u8]) -> Result<SnmpValue, Malformed> {
    match tag {
        TAG_NULL if content.is_empty() => Ok(SnmpValue::Null),
        TAG_INTEGER => Ok(SnmpValue::Integer(ber::decode_integer(content)?)),
        TAG_OCTET_STRING => Ok(SnmpValue::OctetString(content.to_vec())),
        TAG_OID => Ok(SnmpValue::ObjectId(Oid::decode(content)?)),
        TAG_COUNTER32 => Ok(SnmpValue::Counter32(decode_u32_app(content)?)),
        TAG_GAUGE32 => Ok(SnmpValue::Gauge32(decode_u32_app(content)?)),
        TAG_TIMETICKS => Ok(SnmpValue::TimeTicks(decode_u32_app(content)?)),
        TAG_NO_SUCH_OBJECT if content.is_empty() => Ok(SnmpValue::NoSuchObject),
        TAG_NO_SUCH_INSTANCE if content.is_empty() => Ok(SnmpValue::NoSuchInstance),
        TAG_END_OF_MIB_VIEW if content.is_empty() => Ok(SnmpValue::EndOfMibView),
        _ => Err(Malformed),
    }
}

fn encode_value(value: &SnmpValue, out: &mut Vec<u8>) -> Result<(), Malformed> {
    match value {
        SnmpValue::Null => ber::write_tlv(TAG_NULL, &[], out),
        SnmpValue::Integer(v) => ber::write_tlv(TAG_INTEGER, &ber::encode_integer(*v), out),
        SnmpValue::OctetString(b) => ber::write_tlv(TAG_OCTET_STRING, b, out),
        SnmpValue::ObjectId(oid) => ber::write_tlv(TAG_OID, &oid.encode()?, out),
        SnmpValue::Counter32(v) => ber::write_tlv(TAG_COUNTER32, &ber::encode_unsigned(*v), out),
        SnmpValue::Gauge32(v) => ber::write_tlv(TAG_GAUGE32, &ber::encode_unsigned(*v), out),
        SnmpValue::TimeTicks(t) => ber::write_tlv(TAG_TIMETICKS, &ber::encode_unsigned(*t), out),
        SnmpValue::NoSuchObject => ber::write_tlv(TAG_NO_SUCH_OBJECT, &[], out),
        SnmpValue::NoSuchInstance => ber::write_tlv(TAG_NO_SUCH_INSTANCE, &[], out),
        SnmpValue::EndOfMibView => ber::write_tlv(TAG_END_OF_MIB_VIEW, &[], out),
    }
    Ok(())
}

fn encode_pdu(pdu: &Pdu) -> Result<Vec<u8>, Malformed> {
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
    ber::write_tlv(pdu.pdu_type as u8, &body, &mut out);
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
        assert!(matches!(err, Malformed));
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
        assert_eq!(PduType::GetNextRequest as u8, 0xa1);
        assert_eq!(PduType::SetRequest as u8, 0xa3);
        assert_eq!(PduType::GetBulkRequest as u8, 0xa5);
        assert_eq!(PduType::from_tag(0xa1), Some(PduType::GetNextRequest));
        assert_eq!(PduType::from_tag(0xa3), Some(PduType::SetRequest));
        assert_eq!(PduType::from_tag(0xa5), Some(PduType::GetBulkRequest));
        assert_eq!(PduType::from_tag(0x99), None);
    }

    #[test]
    fn test_parse_rejects_trailing_bytes_and_bad_community_utf8() {
        let mut bytes = hand_built_get_sysdescr();
        bytes.push(0x00);
        assert!(matches!(SnmpMessage::parse(&bytes), Err(Malformed)));

        let mut bad = hand_built_get_sysdescr();
        // community bytes start at index 7 for "public"
        bad[7] = 0xff;
        assert!(matches!(SnmpMessage::parse(&bad), Err(Malformed)));
    }

    #[test]
    fn test_decode_value_rejects_unknown_tag_and_oversized_unsigned() {
        assert!(matches!(decode_value(0x99, &[]), Err(Malformed)));
        assert_eq!(decode_u32_app(&[0xff]).unwrap(), 255);
        assert!(matches!(
            decode_u32_app(&[0x01, 0, 0, 0, 0]),
            Err(Malformed)
        ));
        assert!(matches!(
            decode_u32_app(&[0, 0, 0, 0, 0, 0]),
            Err(Malformed)
        ));
    }

    #[test]
    fn test_counter32_max_round_trips() {
        let oid = Oid::from_slice(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 10, 1]).unwrap();
        let msg = SnmpMessage {
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

    #[test]
    fn test_exception_values_round_trip() {
        for (value, tag) in [
            (SnmpValue::NoSuchObject, 0x80u8),
            (SnmpValue::NoSuchInstance, 0x81),
            (SnmpValue::EndOfMibView, 0x82),
        ] {
            let mut out = Vec::new();
            encode_value(&value, &mut out).unwrap();
            assert_eq!(out, vec![tag, 0x00], "{value:?} must be a zero-length TLV");
            assert_eq!(decode_value(tag, &[]).unwrap(), value);
        }
    }
}
