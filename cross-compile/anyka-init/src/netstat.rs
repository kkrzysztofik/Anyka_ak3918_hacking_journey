//! Link-state readers for the wifi monitor.
//!
//! Every function here is a pure parse over a `/proc` or `/sys` file's
//! contents, so the whole health check is provable on the host.

use std::net::{Ipv4Addr, UdpSocket};
use std::time::Duration;

/// `/sys/class/net/<if>/operstate`. Only `up` counts; `dormant` means
/// associated-but-not-ready and `unknown` is what a driver reports when it
/// does not implement the callback.
pub fn parse_operstate(src: &str) -> bool {
    src.trim() == "up"
}

/// Extract the default gateway for `iface` from `/proc/net/route`.
///
/// Read from the kernel rather than from `[wifi].gateway` on purpose: it works
/// identically for DHCP and static, and the monitor can never probe an address
/// the kernel is not actually routing through.
pub fn parse_default_route(src: &str, iface: &str) -> Option<String> {
    for line in src.lines().skip(1) {
        let mut f = line.split_whitespace();
        let (Some(dev), Some(dest), Some(gw)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        if dev != iface || dest != "00000000" {
            continue;
        }
        let raw = u32::from_str_radix(gw, 16).ok()?;
        if raw == 0 {
            continue;
        }
        // /proc/net/route stores addresses little-endian.
        return Some(Ipv4Addr::from(raw.swap_bytes()).to_string());
    }
    None
}

/// ATF_COM: the ARP entry is resolved, so the gateway answered at L2.
const ATF_COM: u32 = 0x2;

/// Does `/proc/net/arp` hold a complete entry for `addr`?
pub fn arp_entry_complete(src: &str, addr: &str) -> bool {
    for line in src.lines().skip(1) {
        let mut f = line.split_whitespace();
        let (Some(ip), Some(_hw_type), Some(flags)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        if ip != addr {
            continue;
        }
        let flags = flags
            .strip_prefix("0x")
            .and_then(|h| u32::from_str_radix(h, 16).ok())
            .unwrap_or(0);
        return flags & ATF_COM != 0;
    }
    false
}

/// Poke the gateway so the kernel resolves it, then read the ARP table.
///
/// Port 9 is `discard`. Nothing needs to be listening — the UDP send is only
/// there to force address resolution. A `Result` from `send_to` is ignored for
/// the same reason.
pub fn gateway_reachable(gw: &str) -> bool {
    if let Ok(sock) = UdpSocket::bind("0.0.0.0:0") {
        let _ = sock.set_write_timeout(Some(Duration::from_millis(200)));
        let _ = sock.send_to(&[0u8; 1], format!("{gw}:9"));
    }
    std::thread::sleep(Duration::from_millis(200));
    std::fs::read_to_string("/proc/net/arp")
        .map(|src| arp_entry_complete(&src, gw))
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Health {
    pub carrier: bool,
    pub route: bool,
    /// Only meaningful when `carrier && route`; the caller does not probe
    /// otherwise.
    pub reachable: bool,
}

impl Health {
    pub fn ok(&self) -> bool {
        self.carrier && self.route && self.reachable
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub dhcp_after_ticks: u32,
    pub supplicant_after_ticks: u32,
    pub reboot_after_ticks: u32,
    pub reboot_cap: u8,
    /// Persisted across reboots; zeroed only by a successful `bring_up`.
    pub wifi_reboots_used: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Nothing,
    RunDhcp,
    RestartSupplicant,
    Reboot,
    /// Escalation exhausted. Keep logging; keep serving video locally.
    LogOnly,
}

/// Host IPv4 for `iface`.
///
/// `/proc/net/fib_trie` carries the host addresses but no interface, and
/// `/proc/net/route` carries the interface but only network addresses. Joining
/// them gives an interface-attributed host address without an ioctl.
///
/// Both arguments are file *contents*, never paths, so this is provable on the
/// host.
pub fn parse_local_ipv4(fib_trie: &str, route: &str, iface: &str) -> Option<String> {
    let subnets = iface_subnets(route, iface);
    if subnets.is_empty() {
        return None;
    }
    for addr in local_host_addresses(fib_trie) {
        if addr.octets()[0] == 127 {
            continue;
        }
        let raw = u32::from(addr);
        for &(network, mask) in &subnets {
            if raw & mask == network {
                return Some(addr.to_string());
            }
        }
    }
    None
}

fn decode_le_hex(hex: &str) -> Option<u32> {
    let raw = u32::from_str_radix(hex, 16).ok()?;
    Some(raw.swap_bytes())
}

/// Non-default routes for `iface`: `(network, mask)` as host-order u32.
fn iface_subnets(route: &str, iface: &str) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for line in route.lines().skip(1) {
        let mut f = line.split_whitespace();
        let (Some(dev), Some(dest), Some(_gw)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        // Flags RefCnt Use Metric Mask
        let (_flags, _refcnt, _use, _metric, Some(mask_hex)) =
            (f.next(), f.next(), f.next(), f.next(), f.next())
        else {
            continue;
        };
        if dev != iface || dest == "00000000" {
            continue;
        }
        let (Some(network), Some(mask)) = (decode_le_hex(dest), decode_le_hex(mask_hex)) else {
            continue;
        };
        out.push((network, mask));
    }
    out
}

/// `/32 host LOCAL` addresses from the `Local:` section of fib_trie.
fn local_host_addresses(fib_trie: &str) -> Vec<Ipv4Addr> {
    let mut out = Vec::new();
    let mut in_local = false;
    let mut pending: Option<Ipv4Addr> = None;
    for line in fib_trie.lines() {
        if line.contains("Local:") {
            in_local = true;
            pending = None;
            continue;
        }
        if line.starts_with("Main:") {
            in_local = false;
            pending = None;
            continue;
        }
        if !in_local {
            continue;
        }
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("|-- ") {
            pending = rest
                .split_whitespace()
                .next()
                .and_then(|a| a.parse::<Ipv4Addr>().ok());
            continue;
        }
        if let Some(addr) = pending.take()
            && trimmed == "/32 host LOCAL"
        {
            out.push(addr);
        }
    }
    out
}

/// The whole recovery ladder. `ticks` counts *consecutive* unhealthy samples
/// and is reset by the caller after any action, so the next escalation starts
/// one rung higher.
pub fn decide(h: Health, ticks: u32, p: &Policy) -> Action {
    if h.ok() {
        return Action::Nothing;
    }
    if ticks >= p.reboot_after_ticks {
        return if p.wifi_reboots_used < p.reboot_cap {
            Action::Reboot
        } else {
            Action::LogOnly
        };
    }
    // DHCP only when associated but unrouted — no carrier needs the
    // supplicant/driver path instead (see test_decide_no_carrier_*).
    if h.carrier && !h.route && ticks >= p.dhcp_after_ticks {
        return Action::RunDhcp;
    }
    if ticks >= p.supplicant_after_ticks {
        return Action::RestartSupplicant;
    }
    Action::Nothing
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real format: `cat /proc/net/route` on a busybox system. Tabs, not spaces.
    const ROUTE: &str = "\
Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
wlan0\t00000000\t0102A8C0\t0003\t0\t0\t0\t00000000\t0\t0\t0
wlan0\t0002A8C0\t00000000\t0001\t0\t0\t0\t00FFFFFF\t0\t0\t0
";

    #[test]
    fn test_parse_default_route_decodes_little_endian_hex_gateway() {
        // 0102A8C0 is 192.168.2.1 stored little-endian.
        let gw = parse_default_route(ROUTE, "wlan0").expect("default route present");
        assert_eq!(gw, "192.168.2.1");
    }

    #[test]
    fn test_parse_default_route_ignores_other_interfaces() {
        assert!(parse_default_route(ROUTE, "eth0").is_none());
    }

    #[test]
    fn test_parse_default_route_none_when_only_subnet_routes() {
        let only_subnet = "Iface\tDestination\tGateway\tFlags\n\
                           wlan0\t0002A8C0\t00000000\t0001\t0\t0\t0\t00FFFFFF\t0\t0\t0\n";
        assert!(parse_default_route(only_subnet, "wlan0").is_none());
    }

    #[test]
    fn test_parse_default_route_handles_empty_and_header_only() {
        assert!(parse_default_route("", "wlan0").is_none());
        assert!(parse_default_route("Iface\tDestination\tGateway\n", "wlan0").is_none());
    }

    #[test]
    fn test_parse_operstate_only_up_is_up() {
        assert!(parse_operstate("up\n"));
        assert!(!parse_operstate("down\n"));
        assert!(!parse_operstate("dormant\n"));
        assert!(!parse_operstate("unknown\n"));
        assert!(!parse_operstate(""));
    }

    const ARP: &str = "\
IP address       HW type     Flags       HW address            Mask     Device
192.168.2.1      0x1         0x2         a4:2b:b0:11:22:33     *        wlan0
192.168.2.50     0x1         0x0         00:00:00:00:00:00     *        wlan0
";

    #[test]
    fn test_parse_arp_complete_entry_is_reachable() {
        // Flags 0x2 is ATF_COM: the entry is resolved.
        assert!(arp_entry_complete(ARP, "192.168.2.1"));
    }

    #[test]
    fn test_parse_arp_incomplete_entry_is_not_reachable() {
        assert!(!arp_entry_complete(ARP, "192.168.2.50"));
    }

    #[test]
    fn test_parse_arp_absent_entry_is_not_reachable() {
        assert!(!arp_entry_complete(ARP, "192.168.2.99"));
        assert!(!arp_entry_complete("", "192.168.2.1"));
    }

    const FIB_TRIE: &str = "\
Main:
  +-- 0.0.0.0/0 3 0 5
     |-- 0.0.0.0
        /0 universe UNICAST
Local:
  +-- 0.0.0.0/0 3 0 4
     |-- 0.0.0.0
        /0 universe UNICAST
     +-- 127.0.0.0/8 2 0 2
        +-- 127.0.0.0/31 1 0 0
           |-- 127.0.0.0
              /8 host LOCAL
           |-- 127.0.0.1
              /32 host LOCAL
        |-- 127.255.255.255
           /32 link BROADCAST
     +-- 192.168.2.0/24 2 0 2
        |-- 192.168.2.0
           /32 link BROADCAST
        |-- 192.168.2.198
           /32 host LOCAL
        |-- 192.168.2.255
           /32 link BROADCAST
";

    #[test]
    fn test_parse_local_ipv4_returns_the_host_address_for_the_interface() {
        let addr = parse_local_ipv4(FIB_TRIE, ROUTE, "wlan0").expect("host address");
        assert_eq!(addr, "192.168.2.198");
    }

    #[test]
    fn test_parse_local_ipv4_never_returns_the_default_route_stub() {
        // F1 regression guard: the old reader returned 0.0.0.0 from Local:.
        let addr = parse_local_ipv4(FIB_TRIE, ROUTE, "wlan0");
        assert_ne!(addr, Some("0.0.0.0".into()));
    }

    #[test]
    fn test_parse_local_ipv4_skips_loopback_and_broadcast() {
        let addr = parse_local_ipv4(FIB_TRIE, ROUTE, "wlan0").expect("host address");
        for bad in ["127.0.0.0", "127.0.0.1", "192.168.2.0", "192.168.2.255"] {
            assert_ne!(addr, bad);
        }
    }

    #[test]
    fn test_parse_local_ipv4_none_for_other_interface() {
        assert!(parse_local_ipv4(FIB_TRIE, ROUTE, "eth0").is_none());
    }

    #[test]
    fn test_parse_local_ipv4_none_when_no_address_assigned() {
        let loopback_only = "\
Local:
  +-- 0.0.0.0/0 3 0 4
     |-- 0.0.0.0
        /0 universe UNICAST
     +-- 127.0.0.0/8 2 0 2
           |-- 127.0.0.1
              /32 host LOCAL
";
        assert!(parse_local_ipv4(loopback_only, ROUTE, "wlan0").is_none());
    }

    #[test]
    fn test_parse_local_ipv4_handles_empty_and_malformed() {
        assert!(parse_local_ipv4("", "", "wlan0").is_none());
        let truncated = "\
Local:
  +-- 192.168.2.0/24 2 0 2
        |-- 192.168.2.198
";
        assert!(parse_local_ipv4(truncated, ROUTE, "wlan0").is_none());
    }

    const POLICY: Policy = Policy {
        dhcp_after_ticks: 3,
        supplicant_after_ticks: 5,
        reboot_after_ticks: 10,
        reboot_cap: 3,
        wifi_reboots_used: 0,
    };

    #[test]
    fn test_decide_healthy_link_does_nothing() {
        let h = Health {
            carrier: true,
            route: true,
            reachable: true,
        };
        assert_eq!(decide(h, 0, &POLICY), Action::Nothing);
        assert_eq!(decide(h, 99, &POLICY), Action::Nothing);
    }

    #[test]
    fn test_decide_waits_before_acting() {
        // A single bad tick is not enough. R16: this also absorbs one stale ARP
        // read.
        let h = Health {
            carrier: true,
            route: false,
            reachable: false,
        };
        assert_eq!(decide(h, 1, &POLICY), Action::Nothing);
        assert_eq!(decide(h, 2, &POLICY), Action::Nothing);
        assert_eq!(decide(h, 3, &POLICY), Action::RunDhcp);
    }

    #[test]
    fn test_decide_missing_route_runs_dhcp() {
        let h = Health {
            carrier: true,
            route: false,
            reachable: false,
        };
        assert_eq!(decide(h, 3, &POLICY), Action::RunDhcp);
    }

    #[test]
    fn test_decide_no_carrier_restarts_supplicant() {
        let h = Health {
            carrier: false,
            route: false,
            reachable: false,
        };
        assert_eq!(decide(h, 5, &POLICY), Action::RestartSupplicant);
    }

    #[test]
    fn test_decide_l3_blackhole_restarts_supplicant() {
        // Associated and addressed but nothing answers: the case operstate alone
        // reports as perfectly healthy.
        let h = Health {
            carrier: true,
            route: true,
            reachable: false,
        };
        assert_eq!(decide(h, 5, &POLICY), Action::RestartSupplicant);
    }

    #[test]
    fn test_decide_escalates_to_reboot_after_the_long_threshold() {
        let h = Health {
            carrier: false,
            route: false,
            reachable: false,
        };
        assert_eq!(decide(h, 10, &POLICY), Action::Reboot);
    }

    #[test]
    fn test_decide_stops_escalating_past_the_reboot_cap() {
        // R14: an AP that is off overnight must not produce an unbounded reboot
        // loop.
        let h = Health {
            carrier: false,
            route: false,
            reachable: false,
        };
        let exhausted = Policy {
            wifi_reboots_used: 3,
            ..POLICY
        };
        assert_eq!(decide(h, 10, &exhausted), Action::LogOnly);
        assert_eq!(decide(h, 1000, &exhausted), Action::LogOnly);
    }
}
