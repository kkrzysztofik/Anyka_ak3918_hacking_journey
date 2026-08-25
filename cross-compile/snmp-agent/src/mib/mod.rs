//! MIB-II object resolution (system + interfaces).

pub mod interfaces;
pub mod system;

use crate::config::SnmpConfig;
use crate::pdu::{PduType, VarBind};

/// SNMP error-status: noError.
pub const ERR_NO_ERROR: i32 = 0;
/// SNMP error-status: noSuchName.
pub const ERR_NO_SUCH_NAME: i32 = 2;
/// SNMP error-status: notWritable for SETs.
pub const ERR_NOT_WRITABLE: i32 = 17;

/// Runtime sources for MIB values.
pub trait MibSources {
    fn uptime_ticks(&self) -> u32;
    fn config(&self) -> &SnmpConfig;
}

fn resolve_get(
    oid: &crate::ber::Oid,
    sources: &dyn MibSources,
) -> Option<(crate::ber::Oid, crate::pdu::SnmpValue)> {
    system::get(oid, sources).or_else(|| interfaces::get(oid, sources))
}

fn resolve_get_next(
    oid: &crate::ber::Oid,
    sources: &dyn MibSources,
) -> Option<(crate::ber::Oid, crate::pdu::SnmpValue)> {
    system::get_next(oid, sources).or_else(|| interfaces::get_next(oid, sources))
}

/// Resolve GET / GETNEXT / reject SET for the fixed OID map.
pub fn handle_varbinds(
    pdu_type: PduType,
    binds: &[VarBind],
    sources: &dyn MibSources,
) -> (i32, i32, Vec<VarBind>) {
    if pdu_type == PduType::SetRequest {
        return (ERR_NOT_WRITABLE, 1, binds.to_vec());
    }
    if pdu_type == PduType::GetResponse {
        return (ERR_NO_SUCH_NAME, 1, binds.to_vec());
    }

    let mut out = Vec::with_capacity(binds.len());
    for (i, vb) in binds.iter().enumerate() {
        let resolved = match pdu_type {
            PduType::GetRequest => resolve_get(&vb.name, sources),
            PduType::GetNextRequest => resolve_get_next(&vb.name, sources),
            PduType::GetResponse | PduType::SetRequest => unreachable!(),
        };
        match resolved {
            Some((oid, value)) => out.push(VarBind { name: oid, value }),
            None => {
                return (ERR_NO_SUCH_NAME, (i + 1) as i32, binds.to_vec());
            }
        }
    }
    (ERR_NO_ERROR, 0, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ber::Oid;
    use crate::config::SnmpConfig;
    use crate::pdu::SnmpValue;

    struct FixedSources {
        cfg: SnmpConfig,
        ticks: u32,
    }

    impl MibSources for FixedSources {
        fn uptime_ticks(&self) -> u32 {
            self.ticks
        }
        fn config(&self) -> &SnmpConfig {
            &self.cfg
        }
    }

    fn sources() -> FixedSources {
        let cfg = SnmpConfig {
            sys_contact: "ops@example".into(),
            sys_name: "cam-1".into(),
            sys_location: "lab".into(),
            ..Default::default()
        };
        FixedSources { cfg, ticks: 42 }
    }

    #[test]
    fn test_get_sys_uptime() {
        let oid = Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 3, 0]).unwrap();
        let (oid_out, val) = system::get(&oid, &sources()).expect("sysUpTime");
        assert_eq!(oid_out, oid);
        assert_eq!(val, SnmpValue::TimeTicks(42));
    }

    #[test]
    fn test_getnext_from_system_prefix_yields_sysdescr() {
        let oid = Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1]).unwrap();
        let (next, _) = system::get_next(&oid, &sources()).expect("next");
        assert_eq!(next.0, vec![1, 3, 6, 1, 2, 1, 1, 1, 0]);
    }

    #[test]
    fn test_system_walk_order() {
        let order = [
            [1, 3, 6, 1, 2, 1, 1, 1, 0], // descr
            [1, 3, 6, 1, 2, 1, 1, 2, 0], // objectID
            [1, 3, 6, 1, 2, 1, 1, 3, 0], // uptime
            [1, 3, 6, 1, 2, 1, 1, 4, 0], // contact
            [1, 3, 6, 1, 2, 1, 1, 5, 0], // name
            [1, 3, 6, 1, 2, 1, 1, 6, 0], // location
            [1, 3, 6, 1, 2, 1, 1, 7, 0], // services
        ];
        let mut cursor = Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1]).unwrap();
        for expected in order {
            let (next, _) = system::get_next(&cursor, &sources()).expect("walk");
            assert_eq!(next.0, expected);
            cursor = next;
        }
        assert!(system::get_next(&cursor, &sources()).is_none());
    }

    #[test]
    fn test_set_returns_not_writable() {
        let oid = Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 5, 0]).unwrap();
        let binds = vec![VarBind {
            name: oid,
            value: SnmpValue::Null,
        }];
        let (status, index, _) = handle_varbinds(PduType::SetRequest, &binds, &sources());
        assert_eq!(status, ERR_NOT_WRITABLE);
        assert_eq!(index, 1);
    }

    #[test]
    fn test_get_all_system_scalars_and_hostname_fallback() {
        let src = sources();
        for arc in 1..=7 {
            let oid = Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, arc, 0]).unwrap();
            assert!(system::get(&oid, &src).is_some(), "scalar {arc}");
        }
        assert!(
            system::get(
                &Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 9, 0]).unwrap(),
                &src
            )
            .is_none()
        );

        let empty_name = FixedSources {
            cfg: SnmpConfig {
                sys_name: String::new(),
                ..Default::default()
            },
            ticks: 1,
        };
        let oid = Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 5, 0]).unwrap();
        let (_, val) = system::get(&oid, &empty_name).unwrap();
        match val {
            SnmpValue::OctetString(b) => assert!(!b.is_empty()),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn test_getnext_and_nosuchname_paths() {
        let binds = vec![VarBind {
            name: Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1]).unwrap(),
            value: SnmpValue::Null,
        }];
        let (status, _, out) = handle_varbinds(PduType::GetNextRequest, &binds, &sources());
        assert_eq!(status, ERR_NO_ERROR);
        assert_eq!(out[0].name.0[7], 1);

        let (status, index, _) = handle_varbinds(PduType::GetResponse, &binds, &sources());
        assert_eq!(status, ERR_NO_SUCH_NAME);
        assert_eq!(index, 1);

        let missing = vec![VarBind {
            name: Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 99, 0]).unwrap(),
            value: SnmpValue::Null,
        }];
        let (status, index, _) = handle_varbinds(PduType::GetRequest, &missing, &sources());
        assert_eq!(status, ERR_NO_SUCH_NAME);
        assert_eq!(index, 1);
    }
}
