// =============================================================================
// Network Info Implementation
// =============================================================================

use std::net::Ipv4Addr;

use crate::config::netoverlay::NetworkOverlay;
use crate::platform::common::{
    DnsInfo, NetworkInfo, NetworkInterfaceInfo, NtpInfo, PlatformError, PlatformResult,
};
use async_trait::async_trait;

const PROC_ROUTE: &str = "/proc/net/route";

/// One parsed row of `/proc/net/route`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RouteEntry {
    iface: String,
    dest: Ipv4Addr,
    gateway: Ipv4Addr,
    mask: Ipv4Addr,
}

/// Parse `/proc/net/route`.
///
/// The kernel prints each address as the native-endian integer view of the
/// network-order bytes, so `to_ne_bytes` recovers the octets on any host —
/// `0002A8C0` is 192.168.2.0, not 0.2.168.192.
pub(super) fn parse_proc_route(text: &str) -> Vec<RouteEntry> {
    fn addr(field: &str) -> Option<Ipv4Addr> {
        Some(Ipv4Addr::from(
            u32::from_str_radix(field, 16).ok()?.to_ne_bytes(),
        ))
    }

    text.lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 8 {
                return None;
            }
            Some(RouteEntry {
                iface: fields[0].to_string(),
                dest: addr(fields[1])?,
                gateway: addr(fields[2])?,
                mask: addr(fields[7])?,
            })
        })
        .collect()
}

/// Gateway of the default route as the kernel currently has it.
///
/// Blocking: reads `/proc/net/route`. Call it from `spawn_blocking`.
fn read_default_gateway() -> Option<String> {
    let routes = std::fs::read_to_string(PROC_ROUTE)
        .map(|text| parse_proc_route(&text))
        .unwrap_or_default();
    default_gateway(&routes)
}

/// Gateway of the default route, if one exists.
pub(super) fn default_gateway(routes: &[RouteEntry]) -> Option<String> {
    routes
        .iter()
        .find(|r| r.dest.is_unspecified() && r.mask.is_unspecified() && !r.gateway.is_unspecified())
        .map(|r| r.gateway.to_string())
}

/// Prefix length of the on-link subnet route that `ip` belongs to on `iface`.
fn subnet_prefix(routes: &[RouteEntry], iface: &str, ip: Ipv4Addr) -> Option<u8> {
    routes
        .iter()
        .find(|r| {
            r.iface == iface
                && !r.mask.is_unspecified()
                && Ipv4Addr::from(u32::from(ip) & u32::from(r.mask)) == r.dest
        })
        .map(|r| u32::from(r.mask).count_ones() as u8)
}

/// True when this `/proc/[pid]/cmdline` is a `udhcpc` bound to `interface`.
///
/// The firmware execs it as `/bin/busybox udhcpc -i wlan0`, so argv[0] alone is
/// not enough to recognise it.
fn cmdline_udhcpc_interface(cmdline: &[u8]) -> Option<String> {
    let args: Vec<&str> = cmdline
        .split(|b| *b == 0)
        .filter(|a| !a.is_empty())
        .filter_map(|a| std::str::from_utf8(a).ok())
        .collect();
    if !args
        .iter()
        .take(2)
        .any(|a| a.rsplit('/').next() == Some("udhcpc"))
    {
        return None;
    }
    args.windows(2)
        .find(|w| w[0] == "-i")
        .map(|w| w[1].to_string())
}

/// Interfaces currently served by a running `udhcpc`.
///
/// There is no lease file and no pidfile on this firmware — busybox writes
/// neither — so the live client process is the only evidence that addressing is
/// dynamic. `anyka-init`'s monitor identifies the same process the same way.
fn udhcpc_interfaces() -> Vec<String> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let cmdline = std::fs::read(entry.path().join("cmdline")).ok()?;
            cmdline_udhcpc_interface(&cmdline)
        })
        .collect()
}

/// Anyka network information implementation.
///
/// Reads network configuration from the Linux system. Falls back to empty
/// values if system files cannot be read.
pub(super) struct AnykaNetworkInfo {
    overlay_path: std::path::PathBuf,
}

impl AnykaNetworkInfo {
    pub(super) fn new() -> Self {
        Self {
            overlay_path: std::path::PathBuf::from(crate::config::netoverlay::DEFAULT_OVERLAY_PATH),
        }
    }

    #[cfg(test)]
    pub(super) fn with_overlay_path(overlay_path: std::path::PathBuf) -> Self {
        Self { overlay_path }
    }

    /// Read the overlay, hand it to `edit`, write it back.
    ///
    /// Read-modify-write because each ONVIF setter owns a different slice of
    /// the same file; a blind write would drop the other setters' work.
    fn update_overlay(&self, edit: impl FnOnce(&mut NetworkOverlay)) -> PlatformResult<()> {
        NetworkOverlay::update_at(&self.overlay_path, edit)
            .map_err(|e| PlatformError::HardwareFailure(e.to_string()))
    }

    /// Read network interfaces from /sys/class/net and /proc/net/route.
    ///
    /// Blocking: walks `/sys/class/net` and all of `/proc`. Call it from
    /// `spawn_blocking`, not straight from an async handler.
    pub(super) fn read_interfaces(local_ip: Option<Ipv4Addr>) -> Vec<NetworkInterfaceInfo> {
        use std::fs;
        use std::path::Path;

        let net_dir = Path::new("/sys/class/net");
        let mut interfaces = Vec::new();

        // Read once, not per interface: both are whole-directory walks.
        let routes = fs::read_to_string(PROC_ROUTE)
            .map(|text| parse_proc_route(&text))
            .unwrap_or_default();
        let dhcp_ifaces = udhcpc_interfaces();

        // Try to read available interfaces
        if let Ok(entries) = fs::read_dir(net_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip loopback
                if name == "lo" {
                    continue;
                }

                // Read MAC address
                let mac_path = entry.path().join("address");
                let mac_address = fs::read_to_string(&mac_path)
                    .ok()
                    .map(|s| s.trim().to_uppercase());

                // Read operational state
                let operstate_path = entry.path().join("operstate");
                let enabled = fs::read_to_string(&operstate_path)
                    .map(|s| s.trim() == "up")
                    .unwrap_or(false);

                // Read link speed (in Mbps)
                let speed_path = entry.path().join("speed");
                let link_speed = fs::read_to_string(&speed_path)
                    .ok()
                    .and_then(|s| s.trim().parse::<u32>().ok());

                let (ipv4_address, ipv4_prefix_length) =
                    Self::interface_address(&routes, &name, local_ip);
                let ipv4_dhcp = dhcp_ifaces.contains(&name);

                interfaces.push(NetworkInterfaceInfo {
                    token: name.clone(),
                    name,
                    enabled,
                    ipv4_address,
                    ipv4_prefix_length,
                    ipv4_dhcp,
                    mac_address,
                    link_speed,
                });
            }
        }

        interfaces
    }

    /// IPv4 address and prefix length of `interface`.
    ///
    /// `local_ip` is the outbound source address (the UDP-connect trick). It is
    /// claimed by whichever interface owns an on-link route it falls inside,
    /// which also yields the prefix from that route's netmask.
    ///
    // ponytail: only the interface carrying the outbound route gets an address;
    // a second, non-default-route NIC still reports None. Switch to getifaddrs
    // if these cameras ever become multi-homed.
    pub(super) fn interface_address(
        routes: &[RouteEntry],
        interface: &str,
        local_ip: Option<Ipv4Addr>,
    ) -> (Option<String>, Option<u8>) {
        let Some(ip) = local_ip else {
            return (None, None);
        };
        match subnet_prefix(routes, interface, ip) {
            Some(prefix) => (Some(ip.to_string()), Some(prefix)),
            None => (None, None),
        }
    }

    /// Read DNS configuration from /etc/resolv.conf.
    ///
    /// Blocking: `udhcpc_interfaces()` walks all of `/proc`.
    pub(super) fn read_dns_config() -> DnsInfo {
        use std::fs;

        let mut dns_info = DnsInfo::default();

        if let Ok(content) = fs::read_to_string("/etc/resolv.conf") {
            for line in content.lines() {
                let line = line.trim();

                // Skip comments
                if line.starts_with('#') {
                    continue;
                }

                if let Some(domain) = line.strip_prefix("search ") {
                    dns_info
                        .search_domains
                        .extend(domain.split_whitespace().map(String::from));
                } else if let Some(domain) = line.strip_prefix("domain ") {
                    dns_info.search_domains.push(domain.trim().to_string());
                } else if let Some(nameserver) = line.strip_prefix("nameserver ") {
                    let ns = nameserver.trim().to_string();
                    // Assume manual unless we detect DHCP
                    dns_info.dns_manual.push(ns);
                }
            }
        }

        // busybox udhcpc's default.script owns resolv.conf whenever it runs, so
        // a live client means every nameserver in the file came from DHCP.
        if !udhcpc_interfaces().is_empty() {
            dns_info.from_dhcp = true;
            dns_info.dns_from_dhcp = std::mem::take(&mut dns_info.dns_manual);
        }

        dns_info
    }

    /// Read NTP configuration from /etc/ntp.conf or similar.
    pub(super) fn read_ntp_config() -> NtpInfo {
        let mut ntp_info = NtpInfo::default();

        if let Some(servers) = Self::parse_ntp_conf() {
            ntp_info.ntp_manual = servers;
        } else if let Some(servers) = Self::parse_timesyncd_conf() {
            ntp_info.ntp_manual = servers;
        }

        ntp_info
    }

    /// Parse /etc/ntp.conf file.
    pub(super) fn parse_ntp_conf() -> Option<Vec<String>> {
        use std::fs;

        let content = fs::read_to_string("/etc/ntp.conf").ok()?;
        let mut servers = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }

            if let Some(server) = line.strip_prefix("server ") {
                let server = server.split_whitespace().next()?.to_string();
                if !server.is_empty() {
                    servers.push(server);
                }
            }
        }

        if servers.is_empty() {
            None
        } else {
            Some(servers)
        }
    }

    /// Parse /etc/systemd/timesyncd.conf file.
    pub(super) fn parse_timesyncd_conf() -> Option<Vec<String>> {
        use std::fs;

        let content = fs::read_to_string("/etc/systemd/timesyncd.conf").ok()?;
        let mut servers = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if let Some(servers_str) = line.strip_prefix("NTP=") {
                servers.extend(servers_str.split_whitespace().map(String::from));
            }
        }

        if servers.is_empty() {
            None
        } else {
            Some(servers)
        }
    }
}

/// Run a blocking `/proc` or sysfs read off the async runtime.
///
/// Every getter below walks `/proc` (the `udhcpc` scan) or `/sys/class/net`,
/// which is exactly what `diagnostics/processes.rs` warns must not run inline
/// on a runtime thread. These are 30-second WebUI polls, so one task hop costs
/// nothing here — unlike a streaming path, where it would.
async fn off_runtime<T, F>(work: F) -> PlatformResult<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| PlatformError::HardwareFailure(e.to_string()))
}

#[async_trait]
impl NetworkInfo for AnykaNetworkInfo {
    async fn get_network_interfaces(&self) -> PlatformResult<Vec<NetworkInterfaceInfo>> {
        // Resolved out here: the UDP-connect trick sends nothing and needs no
        // blocking pool, and it keeps the closure free of `&self`.
        let local_ip = self.detect_local_ip().and_then(|s| s.parse().ok());
        off_runtime(move || Self::read_interfaces(local_ip)).await
    }

    async fn get_default_gateway(&self) -> PlatformResult<Option<String>> {
        off_runtime(read_default_gateway).await
    }

    async fn get_dns_info(&self) -> PlatformResult<DnsInfo> {
        off_runtime(Self::read_dns_config).await
    }

    async fn get_ntp_info(&self) -> PlatformResult<NtpInfo> {
        Ok(Self::read_ntp_config())
    }

    async fn set_network_interface(
        &self,
        _token: &str,
        ipv4_address: Option<String>,
        ipv4_prefix_length: Option<u8>,
        ipv4_dhcp: bool,
    ) -> PlatformResult<()> {
        // `[wifi]` describes one interface; the token is ignored.
        if !ipv4_dhcp && ipv4_address.is_none() {
            return Err(PlatformError::InvalidParameter(
                "static addressing requires an IPv4 address".into(),
            ));
        }
        self.update_overlay(|o| {
            o.dhcp = Some(ipv4_dhcp);
            o.address = if ipv4_dhcp {
                None
            } else {
                ipv4_address.map(|a| format!("{a}/{}", ipv4_prefix_length.unwrap_or(24)))
            };
        })
    }

    async fn set_dns(
        &self,
        dns_servers: &[String],
        _search_domains: &[String],
    ) -> PlatformResult<()> {
        let servers = dns_servers.to_vec();
        self.update_overlay(|o| o.dns = Some(servers))
    }

    async fn set_gateway(&self, gateway: &str) -> PlatformResult<()> {
        let gateway = gateway.to_string();
        self.update_overlay(|o| o.gateway = Some(gateway))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verbatim from a camera: wlan0 holds 192.168.2.198/24 via 192.168.2.1.
    const PROC_ROUTE_SAMPLE: &str = "\
Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT
wlan0\t00000000\t0102A8C0\t0003\t0\t0\t0\t00000000\t0\t0\t0
wlan0\t0002A8C0\t00000000\t0001\t0\t0\t0\t00FFFFFF\t0\t0\t0
";

    #[test]
    fn test_parse_proc_route_decodes_native_endian_addresses() {
        let routes = parse_proc_route(PROC_ROUTE_SAMPLE);
        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0].gateway, Ipv4Addr::new(192, 168, 2, 1));
        assert_eq!(routes[1].dest, Ipv4Addr::new(192, 168, 2, 0));
        assert_eq!(routes[1].mask, Ipv4Addr::new(255, 255, 255, 0));
    }

    #[test]
    fn test_default_gateway_picks_the_zero_route() {
        assert_eq!(
            default_gateway(&parse_proc_route(PROC_ROUTE_SAMPLE)).as_deref(),
            Some("192.168.2.1")
        );
        assert_eq!(default_gateway(&[]), None);
    }

    #[test]
    fn test_interface_address_claims_the_on_link_subnet() {
        let routes = parse_proc_route(PROC_ROUTE_SAMPLE);
        let local = Some(Ipv4Addr::new(192, 168, 2, 198));

        assert_eq!(
            AnykaNetworkInfo::interface_address(&routes, "wlan0", local),
            (Some("192.168.2.198".to_string()), Some(24)),
            "the netmask of the on-link route is the interface prefix"
        );
        assert_eq!(
            AnykaNetworkInfo::interface_address(&routes, "p2p0", local),
            (None, None),
            "an interface with no matching route must not inherit wlan0's address"
        );
        assert_eq!(
            AnykaNetworkInfo::interface_address(&routes, "wlan0", None),
            (None, None)
        );
    }

    #[test]
    fn test_cmdline_udhcpc_interface_recognises_the_busybox_form() {
        let busybox = b"/bin/busybox\0udhcpc\0-i\0wlan0\0-f\0";
        assert_eq!(
            cmdline_udhcpc_interface(busybox).as_deref(),
            Some("wlan0"),
            "argv[0] is busybox on this firmware, so argv[1] has to be checked too"
        );
        assert_eq!(
            cmdline_udhcpc_interface(b"udhcpc\0-i\0wlan0\0").as_deref(),
            Some("wlan0")
        );
        assert_eq!(
            cmdline_udhcpc_interface(b"/usr/sbin/wpa_supplicant\0-i\0wlan0\0"),
            None
        );
        assert_eq!(cmdline_udhcpc_interface(b"udhcpc\0"), None);
    }

    #[test]
    fn test_read_interfaces_skips_loopback_and_invents_no_address() {
        // Runs against the host's real /sys/class/net; only invariants that
        // hold on any Linux box are asserted.
        let interfaces = AnykaNetworkInfo::read_interfaces(None);

        assert!(
            interfaces.iter().all(|i| i.name != "lo"),
            "loopback must never be offered as an ONVIF interface"
        );
        assert!(
            interfaces
                .iter()
                .all(|i| i.ipv4_address.is_none() && i.ipv4_prefix_length.is_none()),
            "with no outbound address known, no interface may claim one"
        );
        assert!(
            interfaces.iter().all(|i| !i.token.is_empty()),
            "an empty token cannot be addressed by SetNetworkInterfaces"
        );
    }

    #[test]
    fn test_read_dns_config_keeps_one_list_authoritative() {
        let dns = AnykaNetworkInfo::read_dns_config();

        assert!(
            !dns.from_dhcp || dns.dns_manual.is_empty(),
            "servers move to dns_from_dhcp wholesale; leaving copies in \
             dns_manual would make GetDNS advertise each one twice"
        );
    }

    #[test]
    fn test_udhcpc_interfaces_walks_proc_without_panicking() {
        // No udhcpc on a build host, so the useful assertion is that the walk
        // survives /proc entries it cannot read (short-lived pids, kthreads).
        assert!(udhcpc_interfaces().iter().all(|i| !i.is_empty()));
    }

    #[tokio::test]
    async fn test_getters_run_the_blocking_reads_off_the_runtime() {
        let info = AnykaNetworkInfo::new();

        // A spawn_blocking panic surfaces as a JoinError, so `is_ok` is the
        // assertion that the /proc and sysfs walks completed on the pool.
        assert!(info.get_network_interfaces().await.is_ok());
        assert!(info.get_default_gateway().await.is_ok());
        assert!(info.get_dns_info().await.is_ok());
    }

    #[tokio::test]
    async fn test_set_network_interface_writes_static_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");
        let info = AnykaNetworkInfo::with_overlay_path(path.clone());

        info.set_network_interface("eth0", Some("192.168.2.50".into()), Some(24), false)
            .await
            .expect("set must succeed");

        let overlay = NetworkOverlay::read(&path).expect("read");
        assert_eq!(overlay.dhcp, Some(false));
        assert_eq!(
            overlay.address.as_deref(),
            Some("192.168.2.50/24"),
            "ONVIF sends address and prefix separately; the overlay stores CIDR"
        );
    }

    #[tokio::test]
    async fn test_set_dns_preserves_an_existing_address() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");
        let info = AnykaNetworkInfo::with_overlay_path(path.clone());

        info.set_network_interface("eth0", Some("192.168.2.50".into()), Some(24), false)
            .await
            .expect("set interface");
        info.set_dns(&["1.1.1.1".to_string()], &[])
            .await
            .expect("set dns");

        let overlay = NetworkOverlay::read(&path).expect("read");
        assert_eq!(overlay.dns, Some(vec!["1.1.1.1".to_string()]));
        assert_eq!(
            overlay.address.as_deref(),
            Some("192.168.2.50/24"),
            "SetDNS must not drop what SetNetworkInterfaces wrote"
        );
    }

    #[tokio::test]
    async fn test_set_dhcp_clears_the_static_address() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");
        let info = AnykaNetworkInfo::with_overlay_path(path.clone());

        info.set_network_interface("eth0", Some("192.168.2.50".into()), Some(24), false)
            .await
            .expect("set static");
        info.set_network_interface("eth0", None, None, true)
            .await
            .expect("set dhcp");

        let overlay = NetworkOverlay::read(&path).expect("read");
        assert_eq!(overlay.dhcp, Some(true));
        assert!(
            overlay.address.is_none(),
            "a stale address left behind DHCP would be applied on the next switch back to static"
        );
    }
}
