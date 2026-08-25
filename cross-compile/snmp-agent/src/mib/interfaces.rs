//! MIB-II interfaces group from `/proc/net/dev`.

use crate::ber::Oid;
use crate::mib::MibSources;
use crate::pdu::SnmpValue;
use std::path::Path;

/// One row of ifTable derived from `/proc/net/dev`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfRow {
    pub index: u32,
    pub descr: String,
    pub in_octets: u32,
    pub out_octets: u32,
}

/// Parse `/proc/net/dev` text into ifTable rows (1-based ifIndex order).
pub fn parse_proc_net_dev(text: &str) -> Vec<IfRow> {
    let mut rows = Vec::new();
    for line in text.lines().skip(2) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_string();
        let fields: Vec<&str> = rest.split_whitespace().collect();
        if fields.len() < 10 {
            continue;
        }
        let in_octets = fields[0].parse().unwrap_or(0);
        let out_octets = fields[8].parse().unwrap_or(0);
        rows.push(IfRow {
            index: (rows.len() as u32) + 1,
            descr: name,
            in_octets,
            out_octets,
        });
    }
    rows
}

pub fn load_interfaces(path: impl AsRef<Path>) -> Vec<IfRow> {
    std::fs::read_to_string(path.as_ref())
        .map(|t| parse_proc_net_dev(&t))
        .unwrap_or_default()
}

fn if_number_oid() -> Oid {
    Oid::from_slice(&[1, 3, 6, 1, 2, 1, 2, 1, 0]).unwrap()
}

/// Columnar OIDs: ifIndex(1), ifDescr(2), ifType(3), ifMtu(4), ifSpeed(5),
/// ifPhysAddress(6), ifAdminStatus(7), ifOperStatus(8), ifInOctets(10), ifOutOctets(16).
fn column_ids() -> [u32; 10] {
    [1, 2, 3, 4, 5, 6, 7, 8, 10, 16]
}

fn table_oid(column: u32, index: u32) -> Oid {
    Oid::from_slice(&[1, 3, 6, 1, 2, 1, 2, 2, 1, column, index]).unwrap()
}

fn all_table_oids(rows: &[IfRow]) -> Vec<Oid> {
    let mut out = Vec::new();
    out.push(if_number_oid());
    for col in column_ids() {
        for row in rows {
            out.push(table_oid(col, row.index));
        }
    }
    out
}

fn cell_value(column: u32, row: &IfRow) -> SnmpValue {
    match column {
        1 => SnmpValue::Integer(row.index as i32),
        2 => SnmpValue::OctetString(row.descr.as_bytes().to_vec()),
        3 => SnmpValue::Integer(6), // ethernetCsmacd (approx for all)
        4 => SnmpValue::Integer(1500),
        5 => SnmpValue::Gauge32(0),          // unknown
        6 => SnmpValue::OctetString(vec![]), // empty phys address in v1
        7 => SnmpValue::Integer(1),          // up
        8 => SnmpValue::Integer(1),          // up
        10 => SnmpValue::Counter32(row.in_octets),
        16 => SnmpValue::Counter32(row.out_octets),
        _ => SnmpValue::Null,
    }
}

fn value_for(oid: &Oid, rows: &[IfRow]) -> Option<SnmpValue> {
    if oid == &if_number_oid() {
        return Some(SnmpValue::Integer(rows.len() as i32));
    }
    // 1.3.6.1.2.1.2.2.1.<col>.<idx>
    if oid.0.len() == 11 && oid.0[..9] == [1, 3, 6, 1, 2, 1, 2, 2, 1] {
        let col = oid.0[9];
        let idx = oid.0[10];
        let row = rows.iter().find(|r| r.index == idx)?;
        return Some(cell_value(col, row));
    }
    None
}

fn oid_less(a: &Oid, b: &Oid) -> bool {
    a.0.iter().cmp(b.0.iter()).is_lt()
}

/// Exact GET against interfaces MIB using injected rows.
pub fn get_with_rows(oid: &Oid, rows: &[IfRow]) -> Option<(Oid, SnmpValue)> {
    let value = value_for(oid, rows)?;
    Some((oid.clone(), value))
}

pub fn get_next_with_rows(oid: &Oid, rows: &[IfRow]) -> Option<(Oid, SnmpValue)> {
    for candidate in all_table_oids(rows) {
        if oid_less(oid, &candidate) {
            let value = value_for(&candidate, rows)?;
            return Some((candidate, value));
        }
    }
    None
}

/// GET using live `/proc/net/dev` (for agent runtime).
pub fn get(oid: &Oid, _sources: &dyn MibSources) -> Option<(Oid, SnmpValue)> {
    let rows = load_interfaces("/proc/net/dev");
    get_with_rows(oid, &rows)
}

pub fn get_next(oid: &Oid, _sources: &dyn MibSources) -> Option<(Oid, SnmpValue)> {
    let rows = load_interfaces("/proc/net/dev");
    get_next_with_rows(oid, &rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_rows() -> Vec<IfRow> {
        let text = include_str!("../../tests/fixtures/proc_net_dev.txt");
        parse_proc_net_dev(text)
    }

    #[test]
    fn test_parse_fixture_if_number_and_descr() {
        let rows = fixture_rows();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].descr, "lo");
        assert_eq!(rows[1].descr, "eth0");
        assert_eq!(rows[2].descr, "wlan0");
        assert_eq!(rows[1].in_octets, 10000);
        assert_eq!(rows[1].out_octets, 20000);
    }

    #[test]
    fn test_get_if_number() {
        let rows = fixture_rows();
        let oid = if_number_oid();
        let (_, val) = get_with_rows(&oid, &rows).unwrap();
        assert_eq!(val, SnmpValue::Integer(3));
    }

    #[test]
    fn test_get_if_descr_wlan0() {
        let rows = fixture_rows();
        let oid = table_oid(2, 3);
        let (_, val) = get_with_rows(&oid, &rows).unwrap();
        assert_eq!(val, SnmpValue::OctetString(b"wlan0".to_vec()));
    }

    #[test]
    fn test_get_if_in_octets_eth0() {
        let rows = fixture_rows();
        let oid = table_oid(10, 2);
        let (_, val) = get_with_rows(&oid, &rows).unwrap();
        assert_eq!(val, SnmpValue::Counter32(10000));
    }

    #[test]
    fn test_getnext_from_interfaces_prefix() {
        let rows = fixture_rows();
        let oid = Oid::from_slice(&[1, 3, 6, 1, 2, 1, 2]).unwrap();
        let (next, _) = get_next_with_rows(&oid, &rows).unwrap();
        assert_eq!(next, if_number_oid());
    }

    #[test]
    fn test_parse_skips_blank_and_malformed_lines() {
        let text = "Inter-|   Receive\n face |bytes\n\nlo: 1 0 0 0 0 0 0 0 2 0\nbadline\neth0: 1\nwlan0: 10 0 0 0 0 0 0 0 20 0 0\n";
        let rows = parse_proc_net_dev(text);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].descr, "lo");
        assert_eq!(rows[1].descr, "wlan0");
    }

    #[test]
    fn test_load_interfaces_missing_file_empty() {
        assert!(load_interfaces("/no/such/proc_net_dev").is_empty());
    }

    #[test]
    fn test_get_table_columns_and_unknown_oid() {
        let rows = fixture_rows();
        let (_, v) = get_with_rows(&table_oid(1, 1), &rows).unwrap();
        assert_eq!(v, SnmpValue::Integer(1));
        let (_, v) = get_with_rows(&table_oid(3, 1), &rows).unwrap();
        assert_eq!(v, SnmpValue::Integer(6));
        let (_, v) = get_with_rows(&table_oid(5, 1), &rows).unwrap();
        assert_eq!(v, SnmpValue::Gauge32(0));
        let (_, v) = get_with_rows(&table_oid(6, 1), &rows).unwrap();
        assert_eq!(v, SnmpValue::OctetString(vec![]));
        let (_, v) = get_with_rows(&table_oid(7, 1), &rows).unwrap();
        assert_eq!(v, SnmpValue::Integer(1));
        let (_, v) = get_with_rows(&table_oid(8, 1), &rows).unwrap();
        assert_eq!(v, SnmpValue::Integer(1));
        let (_, v) = get_with_rows(&table_oid(16, 2), &rows).unwrap();
        assert_eq!(v, SnmpValue::Counter32(20000));
        assert!(get_with_rows(&Oid::from_slice(&[1, 2, 3]).unwrap(), &rows).is_none());
        assert!(get_with_rows(&table_oid(2, 99), &rows).is_none());
        assert!(get_next_with_rows(&table_oid(16, 3), &rows).is_none());
    }

    #[test]
    fn test_live_proc_get_if_number_smoke() {
        // Exercise /proc/net_dev loaders on the CI host.
        let oid = if_number_oid();
        struct Dummy;
        impl MibSources for Dummy {
            fn uptime_ticks(&self) -> u32 {
                0
            }
            fn config(&self) -> &crate::config::SnmpConfig {
                use std::sync::LazyLock;
                static CFG: LazyLock<crate::config::SnmpConfig> =
                    LazyLock::new(crate::config::SnmpConfig::default);
                &CFG
            }
        }
        let got = get(&oid, &Dummy);
        assert!(got.is_some());
        let prefix = Oid::from_slice(&[1, 3, 6, 1, 2, 1, 2]).unwrap();
        assert!(get_next(&prefix, &Dummy).is_some());
    }
}
