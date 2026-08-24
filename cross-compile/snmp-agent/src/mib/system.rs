//! MIB-II system group (1.3.6.1.2.1.1).

use crate::ber::Oid;
use crate::mib::MibSources;
use crate::pdu::SnmpValue;

/// sysServices: application layer (bit 6) typical for a camera/app agent.
const SYS_SERVICES: i32 = 72;

/// Build-time identity; not user-editable in v1.
pub const SYS_DESCR: &str = "Anyka AK3918 IP camera (snmp-agent)";
/// Private enterprise placeholder under .1.3.6.1.4.1.0 until a real PEN is registered.
pub fn sys_object_id() -> Oid {
    Oid::from_slice(&[1, 3, 6, 1, 4, 1, 0, 1]).expect("sysObjectID")
}

fn system_scalars() -> [Oid; 7] {
    [
        Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 1, 0]).unwrap(),
        Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 2, 0]).unwrap(),
        Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 3, 0]).unwrap(),
        Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 4, 0]).unwrap(),
        Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 5, 0]).unwrap(),
        Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 6, 0]).unwrap(),
        Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 7, 0]).unwrap(),
    ]
}

fn value_for(oid: &Oid, sources: &dyn MibSources) -> Option<SnmpValue> {
    let arcs = &oid.0;
    if arcs.len() != 9 || arcs[..7] != [1, 3, 6, 1, 2, 1, 1] || arcs[8] != 0 {
        return None;
    }
    let cfg = sources.config();
    match arcs[7] {
        1 => Some(SnmpValue::OctetString(SYS_DESCR.as_bytes().to_vec())),
        2 => Some(SnmpValue::ObjectId(sys_object_id())),
        3 => Some(SnmpValue::TimeTicks(sources.uptime_ticks())),
        4 => Some(SnmpValue::OctetString(cfg.sys_contact.as_bytes().to_vec())),
        5 => {
            let name = if cfg.sys_name.is_empty() {
                hostname_fallback()
            } else {
                cfg.sys_name.clone()
            };
            Some(SnmpValue::OctetString(name.into_bytes()))
        }
        6 => Some(SnmpValue::OctetString(cfg.sys_location.as_bytes().to_vec())),
        7 => Some(SnmpValue::Integer(SYS_SERVICES)),
        _ => None,
    }
}

fn hostname_fallback() -> String {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "anyka".to_string())
}

/// Exact GET for a system scalar.
pub fn get(oid: &Oid, sources: &dyn MibSources) -> Option<(Oid, SnmpValue)> {
    let value = value_for(oid, sources)?;
    Some((oid.clone(), value))
}

/// Lexicographic next system scalar after `oid`.
pub fn get_next(oid: &Oid, sources: &dyn MibSources) -> Option<(Oid, SnmpValue)> {
    for candidate in system_scalars() {
        if oid_less(oid, &candidate) {
            let value = value_for(&candidate, sources)?;
            return Some((candidate, value));
        }
    }
    None
}

fn oid_less(a: &Oid, b: &Oid) -> bool {
    a.0.iter().cmp(b.0.iter()).is_lt()
}
