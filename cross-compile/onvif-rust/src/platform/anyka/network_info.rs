// =============================================================================
// Network Info Implementation
// =============================================================================

use crate::platform::common::{
    DnsInfo, NetworkInfo, NetworkInterfaceInfo, NetworkProtocolInfo, NtpInfo, PlatformResult,
};
use async_trait::async_trait;

/// Anyka network information implementation.
///
/// Reads network configuration from the Linux system. Falls back to empty
/// values if system files cannot be read.
pub(super) struct AnykaNetworkInfo;

impl AnykaNetworkInfo {
    pub(super) fn new() -> Self {
        Self
    }

    /// Read network interfaces from /sys/class/net and /proc/net/route.
    pub(super) fn read_interfaces() -> Vec<NetworkInterfaceInfo> {
        use std::fs;
        use std::path::Path;

        let net_dir = Path::new("/sys/class/net");
        let mut interfaces = Vec::new();

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

                // Try to get IP address via ip command output parsing
                // This is a simplified approach - real implementation might use netlink
                let (ipv4_address, ipv4_prefix_length, ipv4_dhcp) = Self::read_interface_ip(&name);

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

    /// Read IP address for an interface.
    pub(super) fn read_interface_ip(interface: &str) -> (Option<String>, Option<u8>, bool) {
        use std::fs;

        // TODO(github#28): Actually detect IPv4 address using netlink or /proc/net/fib_trie parsing
        // (functional change - out of scope for refactoring PR)
        //
        // Try to read from /etc/network/interfaces or similar
        // This is a simplified check - in real embedded Linux, DHCP state
        // might be determined differently

        // Check if DHCP is used (look for dhclient lease)
        let dhcp_lease_path = format!("/var/lib/dhcp/dhclient.{}.leases", interface);
        let from_dhcp = std::path::Path::new(&dhcp_lease_path).exists();

        // Try reading from /proc/net/fib_trie or parsing ip addr output
        // For now, try a simple approach via /proc/net/route
        if let Ok(route_content) = fs::read_to_string("/proc/net/route") {
            for line in route_content.lines().skip(1) {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() >= 8 && fields[0] == interface {
                    // Parse gateway destination to find interface IP
                    // This is a simplified approach
                    if fields[1] == "00000000" {
                        // Default route - interface has connectivity
                        // Would need more sophisticated parsing for actual IP
                    }
                }
            }
        }

        // For a more complete implementation, we'd use netlink or parse
        // /proc/net/fib_trie, but for now return None (empty will be reported)
        (None, None, from_dhcp)
    }

    /// Read DNS configuration from /etc/resolv.conf.
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

        // Check if DNS was obtained via DHCP
        // Simple heuristic: if /etc/resolv.conf was modified by dhclient
        if std::path::Path::new("/var/lib/dhcp/dhclient.leases").exists() {
            dns_info.from_dhcp = true;
            // Move servers to dhcp list
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

#[async_trait]
impl NetworkInfo for AnykaNetworkInfo {
    async fn get_network_interfaces(&self) -> PlatformResult<Vec<NetworkInterfaceInfo>> {
        Ok(Self::read_interfaces())
    }

    async fn get_dns_info(&self) -> PlatformResult<DnsInfo> {
        Ok(Self::read_dns_config())
    }

    async fn get_ntp_info(&self) -> PlatformResult<NtpInfo> {
        Ok(Self::read_ntp_config())
    }

    async fn get_network_protocols(&self) -> PlatformResult<Vec<NetworkProtocolInfo>> {
        // Return the protocols this ONVIF server supports
        // These are typically configured at build/runtime, not read from system
        Ok(vec![
            NetworkProtocolInfo {
                name: "HTTP".to_string(),
                enabled: true,
                ports: vec![80],
            },
            NetworkProtocolInfo {
                name: "RTSP".to_string(),
                enabled: true,
                ports: vec![554],
            },
        ])
    }
}
