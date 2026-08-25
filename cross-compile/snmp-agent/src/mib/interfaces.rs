//! MIB-II interfaces group from `/proc/net/dev` + `/sys/class/net`.

use crate::ber::Oid;
use crate::mib::MibSources;
use crate::pdu::SnmpValue;
use std::path::Path;

/// One row of ifTable, from `/proc/net/dev` counters plus `/sys/class/net` metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfRow {
    pub index: u32,
    pub descr: String,
    pub if_type: i32,
    pub mtu: i32,
    pub phys_address: Vec<u8>,
    pub admin_status: i32,
    pub oper_status: i32,
    pub in_octets: u32,
    pub out_octets: u32,
}

/// Parse `/proc/net/dev` text into ifTable rows (positional ifIndex until sysfs overlays).
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
            if_type: 1, // other
            mtu: 1500,
            phys_address: Vec::new(),
            admin_status: 1, // ifAdminStatus has no "unknown"; up is the only sane default
            oper_status: 4,  // unknown
            in_octets,
            out_octets,
        });
    }
    rows
}

fn read_trim(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

/// RFC 2863 ifOperStatus from the sysfs `operstate` string.
fn oper_status_code(s: &str) -> i32 {
    match s {
        "up" => 1,
        "down" => 2,
        "testing" => 3,
        "dormant" => 5,
        "notpresent" => 6,
        "lowerlayerdown" => 7,
        _ => 4, // unknown
    }
}

/// ifType from the sysfs ARPHRD value.
fn if_type_code(arphrd: u32) -> i32 {
    match arphrd {
        1 => 6,          // ARPHRD_ETHER -> ethernetCsmacd (wifi presents as this too)
        772 => 24,       // ARPHRD_LOOPBACK -> softwareLoopback
        801..=803 => 71, // ARPHRD_IEEE80211* -> ieee80211
        _ => 1,          // other
    }
}

/// `aa:bb:cc:dd:ee:ff` -> six bytes. An all-zero address means "no address",
/// which RFC 2863 asks us to report as a zero-length octet string.
fn parse_mac(s: &str) -> Vec<u8> {
    let bytes: Vec<u8> = s
        .split(':')
        .filter_map(|b| u8::from_str_radix(b, 16).ok())
        .collect();
    if bytes.len() != 6 || bytes.iter().all(|&b| b == 0) {
        return Vec::new();
    }
    bytes
}

/// Overlay sysfs metadata onto a row. Every field degrades to its default.
fn enrich(row: &mut IfRow, sys_root: &Path) {
    let dir = sys_root.join(&row.descr);
    if let Some(v) = read_trim(&dir.join("ifindex")).and_then(|s| s.parse().ok()) {
        row.index = v;
    }
    if let Some(s) = read_trim(&dir.join("operstate")) {
        row.oper_status = oper_status_code(&s);
    }
    if let Some(s) = read_trim(&dir.join("address")) {
        row.phys_address = parse_mac(&s);
    }
    if let Some(v) = read_trim(&dir.join("mtu")).and_then(|s| s.parse().ok()) {
        row.mtu = v;
    }
    if let Some(v) = read_trim(&dir.join("type")).and_then(|s| s.parse::<u32>().ok()) {
        row.if_type = if_type_code(v);
    }
    if let Some(f) = read_trim(&dir.join("flags"))
        && let Ok(bits) = u32::from_str_radix(f.strip_prefix("0x").unwrap_or(&f), 16)
    {
        row.admin_status = if bits & 0x1 != 0 { 1 } else { 2 }; // IFF_UP
    }
}

/// Read ifTable rows, sorted by kernel ifIndex so an NMS keyed on it stays
/// stable when an interface disappears and returns.
pub fn load_interfaces(proc_net_dev: &Path, sys_root: &Path) -> Vec<IfRow> {
    let text = std::fs::read_to_string(proc_net_dev).unwrap_or_default();
    let mut rows = parse_proc_net_dev(&text);
    for row in &mut rows {
        enrich(row, sys_root);
    }
    rows.sort_by_key(|r| r.index);
    rows
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
        3 => SnmpValue::Integer(row.if_type),
        4 => SnmpValue::Integer(row.mtu),
        5 => SnmpValue::Gauge32(0), // sysfs `speed` is EINVAL on wifi and loopback
        6 => SnmpValue::OctetString(row.phys_address.clone()),
        7 => SnmpValue::Integer(row.admin_status),
        8 => SnmpValue::Integer(row.oper_status),
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

pub fn get(oid: &Oid, sources: &dyn MibSources) -> Option<(Oid, SnmpValue)> {
    get_with_rows(oid, sources.interfaces())
}

pub fn get_next(oid: &Oid, sources: &dyn MibSources) -> Option<(Oid, SnmpValue)> {
    get_next_with_rows(oid, sources.interfaces())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_rows() -> Vec<IfRow> {
        let text = include_str!("../../tests/fixtures/proc_net_dev.txt");
        parse_proc_net_dev(text)
    }

    fn fake_sysfs(root: &Path, name: &str, kv: &[(&str, &str)]) {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        for (k, v) in kv {
            std::fs::write(dir.join(k), v).unwrap();
        }
    }

    /// Writes the shared `/proc/net/dev` fixture and a sysfs tree; returns both roots.
    fn fake_roots(dir: &Path, ifindexes: [&str; 3]) -> (PathBuf, PathBuf) {
        let proc_net_dev = dir.join("net-dev");
        std::fs::write(
            proc_net_dev.as_path(),
            include_str!("../../tests/fixtures/proc_net_dev.txt"),
        )
        .unwrap();
        let sys = dir.join("sys");
        fake_sysfs(
            &sys,
            "lo",
            &[
                ("ifindex", ifindexes[0]),
                ("operstate", "unknown"),
                ("address", "00:00:00:00:00:00"),
                ("mtu", "65536"),
                ("type", "772"),
                ("flags", "0x9"),
            ],
        );
        fake_sysfs(
            &sys,
            "eth0",
            &[
                ("ifindex", ifindexes[1]),
                ("operstate", "down"),
                ("address", "aa:bb:cc:dd:ee:ff"),
                ("mtu", "1500"),
                ("type", "1"),
                ("flags", "0x1002"),
            ],
        );
        fake_sysfs(
            &sys,
            "wlan0",
            &[
                ("ifindex", ifindexes[2]),
                ("operstate", "up"),
                ("address", "11:22:33:44:55:66"),
                ("mtu", "1500"),
                ("type", "1"),
                ("flags", "0x1003"),
            ],
        );
        (proc_net_dev, sys)
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
        let dir = tempfile::tempdir().unwrap();
        assert!(
            load_interfaces(&dir.path().join("missing"), &dir.path().join("no-sysfs")).is_empty()
        );
    }

    #[test]
    fn test_get_table_columns_and_unknown_oid() {
        let rows = fixture_rows();
        let (_, v) = get_with_rows(&table_oid(1, 1), &rows).unwrap();
        assert_eq!(v, SnmpValue::Integer(1));
        let (_, v) = get_with_rows(&table_oid(3, 1), &rows).unwrap();
        assert_eq!(v, SnmpValue::Integer(1)); // other — no sysfs overlay
        let (_, v) = get_with_rows(&table_oid(5, 1), &rows).unwrap();
        assert_eq!(v, SnmpValue::Gauge32(0));
        let (_, v) = get_with_rows(&table_oid(6, 1), &rows).unwrap();
        assert_eq!(v, SnmpValue::OctetString(vec![]));
        let (_, v) = get_with_rows(&table_oid(7, 1), &rows).unwrap();
        assert_eq!(v, SnmpValue::Integer(1));
        let (_, v) = get_with_rows(&table_oid(8, 1), &rows).unwrap();
        assert_eq!(v, SnmpValue::Integer(4)); // unknown — never fabricated up
        let (_, v) = get_with_rows(&table_oid(16, 2), &rows).unwrap();
        assert_eq!(v, SnmpValue::Counter32(20000));
        assert!(get_with_rows(&Oid::from_slice(&[1, 2, 3]).unwrap(), &rows).is_none());
        assert!(get_with_rows(&table_oid(2, 99), &rows).is_none());
        assert!(get_next_with_rows(&table_oid(16, 3), &rows).is_none());
    }

    #[test]
    fn test_sysfs_reports_a_down_interface_as_down() {
        let dir = tempfile::tempdir().unwrap();
        let (proc_net_dev, sys) = fake_roots(dir.path(), ["1", "2", "3"]);
        let rows = load_interfaces(&proc_net_dev, &sys);

        assert_eq!(rows.len(), 3);
        // lo: loopback type, no address, unknown oper state, admin up (0x9 has IFF_UP)
        assert_eq!(rows[0].if_type, 24);
        assert_eq!(rows[0].oper_status, 4);
        assert_eq!(rows[0].phys_address, Vec::<u8>::new());
        assert_eq!(rows[0].mtu, 65536);
        assert_eq!(rows[0].admin_status, 1);
        // eth0 is DOWN — the whole reason this test exists
        assert_eq!(rows[1].oper_status, 2);
        assert_eq!(rows[1].admin_status, 2); // 0x1002 carries no IFF_UP
        assert_eq!(rows[1].if_type, 6);
        assert_eq!(
            rows[1].phys_address,
            vec![0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]
        );
        // wlan0 is up
        assert_eq!(rows[2].oper_status, 1);
        assert_eq!(rows[2].admin_status, 1);
    }

    #[test]
    fn test_rows_sort_by_kernel_ifindex() {
        let dir = tempfile::tempdir().unwrap();
        let (proc_net_dev, sys) = fake_roots(dir.path(), ["1", "5", "3"]);
        let rows = load_interfaces(&proc_net_dev, &sys);
        assert_eq!(
            rows.iter()
                .map(|r| (r.index, r.descr.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "lo"), (3, "wlan0"), (5, "eth0")],
        );
    }

    #[test]
    fn test_missing_sysfs_falls_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let proc_net_dev = dir.path().join("net-dev");
        std::fs::write(
            &proc_net_dev,
            include_str!("../../tests/fixtures/proc_net_dev.txt"),
        )
        .unwrap();
        let rows = load_interfaces(&proc_net_dev, &dir.path().join("no-sysfs"));
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].index, 1); // positional fallback
        assert_eq!(rows[0].oper_status, 4); // unknown, never a fabricated "up"
        assert_eq!(rows[0].mtu, 1500);
    }
}
