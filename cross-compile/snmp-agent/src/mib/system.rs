//! MIB-II system group (1.3.6.1.2.1.1).

use crate::ber::Oid;
use crate::mib::Snapshot;
use crate::pdu::SnmpValue;

/// sysServices: application layer (bit 6) typical for a camera/app agent.
const SYS_SERVICES: i32 = 72;

/// Build-time identity; not user-editable in v1.
pub const SYS_DESCR: &str = "Anyka AK3918 IP camera (snmp-agent)";
/// Private enterprise placeholder under .1.3.6.1.4.1.0 until a real PEN is registered.
pub fn sys_object_id() -> Oid {
    Oid(vec![1, 3, 6, 1, 4, 1, 0, 1])
}

fn value_for(oid: &Oid, sources: &Snapshot) -> Option<SnmpValue> {
    let arcs = &oid.0;
    if arcs.len() != 9 || arcs[..7] != [1, 3, 6, 1, 2, 1, 1] || arcs[8] != 0 {
        return None;
    }
    let cfg = &sources.config;
    match arcs[7] {
        1 => Some(SnmpValue::OctetString(SYS_DESCR.as_bytes().to_vec())),
        2 => Some(SnmpValue::ObjectId(sys_object_id())),
        3 => Some(SnmpValue::TimeTicks(sources.uptime_ticks)),
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
pub fn get(oid: &Oid, sources: &Snapshot) -> Option<(Oid, SnmpValue)> {
    let value = value_for(oid, sources)?;
    Some((oid.clone(), value))
}

/// Lexicographic next system scalar after `oid` (sysDescr.0 … sysServices.0).
pub fn get_next(oid: &Oid, sources: &Snapshot) -> Option<(Oid, SnmpValue)> {
    let next = (1..=7)
        .map(|arc| Oid(vec![1, 3, 6, 1, 2, 1, 1, arc, 0]))
        .find(|candidate| oid < candidate)?;
    let value = value_for(&next, sources)?;
    Some((next, value))
}
