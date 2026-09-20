// =============================================================================
// Network Info Implementation
// =============================================================================

use std::net::{Ipv4Addr, SocketAddrV4};

use socket2::{Domain, Protocol, Socket, Type};

use crate::config::netoverlay::NetworkOverlay;
use crate::platform::common::{
    DnsInfo, NetworkInfo, NetworkInterfaceInfo, NtpInfo, PlatformError, PlatformResult,
};
use async_trait::async_trait;

const PROC_ROUTE: &str = "/proc/net/route";

/// `RTF_UP` — the route is live. Down routes linger in the table.
const RTF_UP: u32 = 0x0001;
/// `RTF_GATEWAY` — the route goes via a next hop rather than being on-link.
const RTF_GATEWAY: u32 = 0x0002;

/// One parsed row of `/proc/net/route`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RouteEntry {
    iface: String,
    dest: Ipv4Addr,
    gateway: Ipv4Addr,
    mask: Ipv4Addr,
    flags: u32,
    metric: u32,
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
                // Flags are hex, metric is decimal — the kernel prints them
                // that way in the same row.
                flags: u32::from_str_radix(fields[3], 16).ok()?,
                metric: fields[6].parse().ok()?,
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

/// Gateway of the default route the kernel would actually use.
///
/// Table order is not precedence: a down route stays listed, and a camera
/// recovering its lease can briefly hold two default routes. The kernel picks
/// the live gateway route with the lowest metric, so this does too — otherwise
/// ONVIF advertises a gateway nothing is routed through.
pub(super) fn default_gateway(routes: &[RouteEntry]) -> Option<String> {
    routes
        .iter()
        .filter(|r| {
            r.dest.is_unspecified()
                && r.mask.is_unspecified()
                && !r.gateway.is_unspecified()
                && r.flags & (RTF_UP | RTF_GATEWAY) == (RTF_UP | RTF_GATEWAY)
        })
        .min_by_key(|r| r.metric)
        .map(|r| r.gateway.to_string())
}

/// An address inside `route`'s on-link subnet, to aim a source lookup at.
///
/// Any address in the prefix does; nothing is ever sent to it. `dest | 1` is
/// the first host, and on a /31 or /32 there is no spare host bit, so the
/// destination itself is the only candidate.
fn probe_address(route: &RouteEntry) -> Option<Ipv4Addr> {
    if route.mask.is_unspecified() {
        return None;
    }
    let dest = u32::from(route.dest);
    let host_bits = u32::from(route.mask).count_zeros();
    Some(Ipv4Addr::from(if host_bits >= 2 { dest | 1 } else { dest }))
}

/// The address `interface` would send from when addressing `probe`.
///
/// `connect` on a UDP socket runs the route lookup and binds a source address
/// without emitting a packet. Aiming it at each on-link subnet in turn, rather
/// than at a fixed public address, is what makes it work on a camera with no
/// default route at all — an isolated VLAN — and on an interface that is not
/// the uplink.
fn source_address_for(interface: &str, probe: Ipv4Addr) -> Option<Ipv4Addr> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).ok()?;

    // Pin the lookup to this interface, so a second one holding a lower-metric
    // route to the same subnet cannot answer in its place. Before Linux 5.7
    // this needs CAP_NET_RAW: onvif-rust runs as root on the camera, and an
    // unprivileged host falls back to an unbound lookup, which the caller's
    // subnet check still guards.
    if let Err(error) = socket.bind_device(Some(interface.as_bytes())) {
        tracing::debug!(
            interface,
            %error,
            "SO_BINDTODEVICE unavailable; source lookup is not pinned"
        );
    }

    socket
        .bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0).into())
        .ok()?;
    socket.connect(&SocketAddrV4::new(probe, 9).into()).ok()?;

    let local = *socket.local_addr().ok()?.as_socket_ipv4()?.ip();
    (!local.is_unspecified()).then_some(local)
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
    ntp_status_path: std::path::PathBuf,
}

impl AnykaNetworkInfo {
    pub(super) fn new() -> Self {
        Self {
            overlay_path: std::path::PathBuf::from(crate::config::netoverlay::DEFAULT_OVERLAY_PATH),
            ntp_status_path: crate::time::ntp_status::path(
                crate::diagnostics::update::DEFAULT_UPDATE_ROOT,
            ),
        }
    }

    #[cfg(test)]
    pub(super) fn with_overlay_path(overlay_path: std::path::PathBuf) -> Self {
        Self {
            overlay_path,
            ..Self::new()
        }
    }

    #[cfg(test)]
    pub(super) fn with_ntp_status_path(ntp_status_path: std::path::PathBuf) -> Self {
        Self {
            ntp_status_path,
            ..Self::new()
        }
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
    pub(super) fn read_interfaces() -> Vec<NetworkInterfaceInfo> {
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

                let (ipv4_address, ipv4_prefix_length) = Self::interface_address(&routes, &name);
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
    /// Each on-link route the interface owns gives the prefix; the kernel then
    /// names the address that interface would send from inside that subnet.
    /// The result is only accepted if it actually falls in the subnet — belt
    /// and braces with the `SO_BINDTODEVICE` pin, and the only guard left when
    /// the kernel refuses that option.
    pub(super) fn interface_address(
        routes: &[RouteEntry],
        interface: &str,
    ) -> (Option<String>, Option<u8>) {
        let on_link = routes.iter().filter(|r| {
            r.iface == interface && !r.mask.is_unspecified() && r.flags & RTF_UP == RTF_UP
        });

        for route in on_link {
            let Some(probe) = probe_address(route) else {
                continue;
            };
            let Some(ip) = source_address_for(interface, probe) else {
                continue;
            };
            if Ipv4Addr::from(u32::from(ip) & u32::from(route.mask)) == route.dest {
                return (
                    Some(ip.to_string()),
                    Some(u32::from(route.mask).count_ones() as u8),
                );
            }
        }
        (None, None)
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

    /// Real servers come from the supervisor's status file.
    ///
    /// There were once `/etc/ntp.conf` and `timesyncd.conf` parsers here.
    /// Neither file exists on this rootfs and nothing called them, so they are
    /// gone; the stub platform returns its own canned `NtpInfo`.
    pub(super) fn read_ntp_config(&self) -> NtpInfo {
        if let Ok(text) = std::fs::read_to_string(&self.ntp_status_path)
            && let Some(status) = crate::time::ntp_status::parse(&text)
        {
            return NtpInfo {
                // udhcpc here never supplies NTP; saying otherwise would be a lie.
                from_dhcp: false,
                ntp_from_dhcp: vec![],
                ntp_manual: status.servers,
            };
        }
        NtpInfo::default()
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
        off_runtime(Self::read_interfaces).await
    }

    async fn get_default_gateway(&self) -> PlatformResult<Option<String>> {
        off_runtime(read_default_gateway).await
    }

    async fn get_dns_info(&self) -> PlatformResult<DnsInfo> {
        off_runtime(Self::read_dns_config).await
    }

    async fn get_ntp_info(&self) -> PlatformResult<NtpInfo> {
        Ok(self.read_ntp_config())
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
    fn test_parse_proc_route_reads_flags_as_hex_and_metric_as_decimal() {
        let routes = parse_proc_route(PROC_ROUTE_SAMPLE);
        assert_eq!(routes[0].flags, RTF_UP | RTF_GATEWAY);
        assert_eq!(routes[1].flags, RTF_UP);
        assert_eq!(routes[0].metric, 0);
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
    fn test_default_gateway_prefers_the_live_lowest_metric_route() {
        // eth0 is down but still listed, wlan0 sits at metric 600, and a
        // freshly renewed wlan0 lease adds a metric-0 route.
        let table = "\
Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT
eth0\t00000000\t0104A8C0\t0002\t0\t0\t0\t00000000\t0\t0\t0
wlan0\t00000000\t0102A8C0\t0003\t0\t0\t600\t00000000\t0\t0\t0
wlan0\t00000000\tFE02A8C0\t0003\t0\t0\t0\t00000000\t0\t0\t0
";
        assert_eq!(
            default_gateway(&parse_proc_route(table)).as_deref(),
            Some("192.168.2.254"),
            "a down route and a higher-metric one must not win over the live default"
        );
    }

    #[test]
    fn test_probe_address_stays_inside_the_prefix() {
        let route = |dest: [u8; 4], mask: [u8; 4]| RouteEntry {
            iface: "wlan0".into(),
            dest: Ipv4Addr::from(dest),
            gateway: Ipv4Addr::UNSPECIFIED,
            mask: Ipv4Addr::from(mask),
            flags: RTF_UP,
            metric: 0,
        };

        assert_eq!(
            probe_address(&route([192, 168, 2, 0], [255, 255, 255, 0])),
            Some(Ipv4Addr::new(192, 168, 2, 1))
        );
        assert_eq!(
            probe_address(&route([10, 0, 0, 7], [255, 255, 255, 255])),
            Some(Ipv4Addr::new(10, 0, 0, 7)),
            "a /32 has no spare host bit, so the destination is the only candidate"
        );
        assert_eq!(
            probe_address(&route([0, 0, 0, 0], [0, 0, 0, 0])),
            None,
            "the default route is not on-link and names no subnet to probe"
        );
    }

    #[test]
    fn test_interface_address_rejects_a_subnet_the_host_is_not_on() {
        // TEST-NET-3 is reserved, so no build host holds an address in it; the
        // source lookup must escape to another interface and be discarded
        // rather than reported as wlan0's address.
        let table = "\
Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT
wlan0\t007100CB\t00000000\t0001\t0\t0\t0\t00FFFFFF\t0\t0\t0
";
        assert_eq!(
            AnykaNetworkInfo::interface_address(&parse_proc_route(table), "wlan0"),
            (None, None)
        );
        assert_eq!(
            AnykaNetworkInfo::interface_address(&parse_proc_route(table), "p2p0"),
            (None, None),
            "an interface with no route of its own must not inherit another's"
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
    fn test_read_interfaces_skips_loopback_and_pairs_address_with_prefix() {
        // Runs against the host's real /sys/class/net; only invariants that
        // hold on any Linux box are asserted.
        let interfaces = AnykaNetworkInfo::read_interfaces();

        assert!(
            interfaces.iter().all(|i| i.name != "lo"),
            "loopback must never be offered as an ONVIF interface"
        );
        assert!(
            interfaces.iter().all(|i| !i.token.is_empty()),
            "an empty token cannot be addressed by SetNetworkInterfaces"
        );
        assert!(
            interfaces
                .iter()
                .all(|i| i.ipv4_address.is_some() == i.ipv4_prefix_length.is_some()),
            "an address without its prefix renders as a bare IP with a made-up /24"
        );
        assert!(
            interfaces
                .iter()
                .filter_map(|i| i.ipv4_address.as_deref())
                .all(|a| a.parse::<Ipv4Addr>().is_ok_and(|ip| !ip.is_loopback())),
            "a reported address must be a real non-loopback IPv4"
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

    #[test]
    fn test_read_ntp_config_reads_the_supervisor_status_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let status = dir.path().join("ntp.status");
        std::fs::write(
            &status,
            "1760000000\tpool.example\t0\tpool.example,192.168.2.1\n",
        )
        .unwrap();
        let info = AnykaNetworkInfo::with_ntp_status_path(status);
        let ntp = info.read_ntp_config();
        assert!(!ntp.from_dhcp);
        assert_eq!(ntp.ntp_manual, vec!["pool.example", "192.168.2.1"]);
    }

    #[test]
    fn test_read_ntp_config_is_empty_when_the_status_file_is_absent() {
        let info = AnykaNetworkInfo::with_ntp_status_path(std::path::PathBuf::from(
            "/nonexistent/update-root/state/ntp.status",
        ));
        assert!(info.read_ntp_config().ntp_manual.is_empty());
    }
}
