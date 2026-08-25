//! Network-related Device Service handlers.
//!
//! This module contains handlers for:
//! - Hostname (GetHostname, SetHostname)
//! - Network interfaces (GetNetworkInterfaces)
//! - DNS (GetDNS, SetDNS)
//! - NTP (GetNTP, SetNTP)
//! - Network gateway (GetNetworkDefaultGateway)
//! - Network protocols (GetNetworkProtocols, SetNetworkProtocols)

use std::sync::Arc;

use crate::config::ConfigRuntime;
use crate::onvif::device::faults::{unsupported_network_config, validate_hostname, validate_ipv4};
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::device::{
    DNSInformation, Duplex, GetDNS, GetDNSResponse, GetHostname, GetHostnameResponse, GetNTP,
    GetNTPResponse, GetNetworkDefaultGateway, GetNetworkDefaultGatewayResponse,
    GetNetworkInterfaces, GetNetworkInterfacesResponse, GetNetworkProtocols,
    GetNetworkProtocolsResponse, HostnameInformation, IPAddress, IPType, IPv4Configuration,
    IPv4NetworkInterface, NTPInformation, NetworkGateway, NetworkHost, NetworkInterface,
    NetworkInterfaceConnectionSetting, NetworkInterfaceInfo, NetworkInterfaceLink, NetworkProtocol,
    NetworkProtocolType, PrefixedIPv4Address, SetDNS, SetDNSResponse, SetHostname,
    SetHostnameResponse, SetNTP, SetNTPResponse, SetNetworkDefaultGateway,
    SetNetworkDefaultGatewayResponse, SetNetworkInterfaces, SetNetworkInterfacesResponse,
    SetNetworkProtocols, SetNetworkProtocolsResponse,
};
use crate::platform::{
    Platform, common::NetworkInterfaceInfo as PlatformInterfaceInfo, external_ip,
};

const IFT_ETHERNET: i32 = 6;
const IFT_IEEE80211: i32 = 71;

/// Handle GetHostname request.
///
/// Returns current hostname configuration.
pub fn handle_get_hostname(
    config: &Arc<ConfigRuntime>,
    _request: GetHostname,
) -> OnvifResult<GetHostnameResponse> {
    tracing::debug!("GetHostname request");

    let hostname = {
        let h = config.read().device.hostname.clone();
        if h.is_empty() {
            "onvif-camera".to_string()
        } else {
            h
        }
    };

    Ok(GetHostnameResponse {
        hostname_information: HostnameInformation {
            from_dhcp: false,
            name: Some(hostname),
            extension: None,
        },
    })
}

/// Handle SetHostname request.
///
/// Sets the device hostname.
pub fn handle_set_hostname(
    config: &Arc<ConfigRuntime>,
    request: SetHostname,
) -> OnvifResult<SetHostnameResponse> {
    tracing::debug!("SetHostname request: {}", request.name);

    // Validate hostname
    validate_hostname(&request.name)?;

    // Save to configuration
    config.write().device.hostname = request.name.clone();

    tracing::info!("SetHostname: hostname set to '{}'", request.name);

    Ok(SetHostnameResponse {})
}

/// Handle GetNetworkInterfaces request.
///
/// Returns network interface configurations from platform or fallback to config.
pub async fn handle_get_network_interfaces(
    platform: &Option<Arc<dyn Platform>>,
    config: &Arc<ConfigRuntime>,
    _request: GetNetworkInterfaces,
) -> OnvifResult<GetNetworkInterfacesResponse> {
    tracing::debug!("GetNetworkInterfaces request");

    if let Some(platform_ref) = platform.as_ref()
        && let Some(network_info) = platform_ref.network_info()
        && let Ok(platform_ifaces) = network_info.get_network_interfaces().await
        && !platform_ifaces.is_empty()
    {
        let network_interfaces = platform_ifaces
            .iter()
            .map(build_onvif_network_interface)
            .collect();
        return Ok(GetNetworkInterfacesResponse { network_interfaces });
    }

    let (ip_address, mac_address, dhcp_enabled) = get_network_info(platform, config).await;
    Ok(GetNetworkInterfacesResponse {
        network_interfaces: vec![build_fallback_network_interface(
            ip_address,
            mac_address,
            dhcp_enabled,
        )],
    })
}

fn onvif_interface_type(name: &str) -> i32 {
    if name.starts_with("wlan") {
        IFT_IEEE80211
    } else {
        IFT_ETHERNET
    }
}

fn admin_link_settings() -> NetworkInterfaceConnectionSetting {
    NetworkInterfaceConnectionSetting {
        auto_negotiation: true,
        speed: 0,
        duplex: Duplex::Full,
    }
}

fn oper_link_settings(speed_mbps: Option<u32>) -> NetworkInterfaceConnectionSetting {
    match speed_mbps {
        Some(speed) => NetworkInterfaceConnectionSetting {
            auto_negotiation: false,
            speed: speed as i32,
            duplex: Duplex::Full,
        },
        None => admin_link_settings(),
    }
}

fn ipv4_configuration(
    ip_address: &str,
    prefix_length: u8,
    dhcp_enabled: bool,
) -> IPv4Configuration {
    if dhcp_enabled {
        IPv4Configuration {
            manual: vec![],
            link_local: None,
            from_dhcp: Some(PrefixedIPv4Address {
                address: ip_address.to_string(),
                prefix_length: i32::from(prefix_length),
            }),
            dhcp: true,
        }
    } else {
        IPv4Configuration {
            manual: vec![PrefixedIPv4Address {
                address: ip_address.to_string(),
                prefix_length: i32::from(prefix_length),
            }],
            link_local: None,
            from_dhcp: None,
            dhcp: false,
        }
    }
}

fn build_onvif_network_interface(iface: &PlatformInterfaceInfo) -> NetworkInterface {
    let ip_address = iface
        .ipv4_address
        .clone()
        .filter(|ip| !ip.is_empty())
        .unwrap_or_default();
    let prefix_length = iface.ipv4_prefix_length.unwrap_or(24);
    let admin_settings = admin_link_settings();
    let oper_settings = oper_link_settings(iface.link_speed);
    let hw_address = iface
        .mac_address
        .clone()
        .unwrap_or_else(|| "00:00:00:00:00:00".to_string());

    NetworkInterface {
        token: iface.token.clone(),
        enabled: iface.enabled,
        info: Some(NetworkInterfaceInfo {
            name: Some(iface.name.clone()),
            hw_address,
            mtu: Some(1500),
        }),
        link: Some(NetworkInterfaceLink {
            admin_settings,
            oper_settings,
            interface_type: onvif_interface_type(&iface.name),
        }),
        ipv4: Some(IPv4NetworkInterface {
            enabled: true,
            config: ipv4_configuration(&ip_address, prefix_length, iface.ipv4_dhcp),
        }),
        ipv6: None,
        extension: None,
    }
}

fn build_fallback_network_interface(
    ip_address: String,
    mac_address: String,
    dhcp_enabled: bool,
) -> NetworkInterface {
    NetworkInterface {
        token: "eth0".to_string(),
        enabled: true,
        info: Some(NetworkInterfaceInfo {
            name: Some("eth0".to_string()),
            hw_address: mac_address,
            mtu: Some(1500),
        }),
        link: Some(NetworkInterfaceLink {
            admin_settings: admin_link_settings(),
            oper_settings: oper_link_settings(Some(100)),
            interface_type: IFT_ETHERNET,
        }),
        ipv4: Some(IPv4NetworkInterface {
            enabled: true,
            config: ipv4_configuration(&ip_address, 24, dhcp_enabled),
        }),
        ipv6: None,
        extension: None,
    }
}

/// Get network info from platform or fallback to config.
async fn get_network_info(
    platform: &Option<Arc<dyn Platform>>,
    config: &Arc<ConfigRuntime>,
) -> (String, String, bool) {
    // Try platform first
    if let Some(platform) = platform
        && let Some(network_info) = platform.network_info()
        && let Ok(interfaces) = network_info.get_network_interfaces().await
        && let Some(iface) = interfaces.first()
    {
        let ip = iface
            .ipv4_address
            .clone()
            .or_else(|| network_info.detect_local_ip())
            .unwrap_or_else(|| "192.168.1.100".to_string());
        let mac = iface
            .mac_address
            .clone()
            .unwrap_or_else(|| "00:00:00:00:00:00".to_string());
        let dhcp = iface.ipv4_dhcp;
        return (ip, mac, dhcp);
    }

    // Fallback to config
    let ip_address = external_ip(config);
    let c = config.read();
    let mac_address = if c.network.mac_address.is_empty() {
        "00:11:22:33:44:55".to_string()
    } else {
        c.network.mac_address.clone()
    };
    let dhcp_enabled = c.network.dhcp_enabled;
    drop(c);

    (ip_address, mac_address, dhcp_enabled)
}

/// Handle GetDNS request.
///
/// Returns DNS configuration.
pub async fn handle_get_dns(
    platform: &Option<Arc<dyn Platform>>,
    _request: GetDNS,
) -> OnvifResult<GetDNSResponse> {
    tracing::debug!("GetDNS request");

    // Try to get DNS info from platform
    if let Some(platform) = platform
        && let Some(network_info) = platform.network_info()
        && let Ok(dns_info) = network_info.get_dns_info().await
    {
        // Convert platform DNS info to ONVIF types
        let dns_from_dhcp: Vec<IPAddress> = dns_info
            .dns_from_dhcp
            .iter()
            .map(|addr| {
                if addr.contains(':') {
                    IPAddress::ipv6(addr)
                } else {
                    IPAddress::ipv4(addr)
                }
            })
            .collect();

        let dns_manual: Vec<IPAddress> = dns_info
            .dns_manual
            .iter()
            .map(|addr| {
                if addr.contains(':') {
                    IPAddress::ipv6(addr)
                } else {
                    IPAddress::ipv4(addr)
                }
            })
            .collect();

        return Ok(GetDNSResponse {
            dns_information: DNSInformation {
                from_dhcp: dns_info.from_dhcp,
                search_domain: dns_info.search_domains,
                dns_from_dhcp,
                dns_manual,
            },
        });
    }

    // Return empty DNS info if no platform available
    Ok(GetDNSResponse {
        dns_information: DNSInformation::default(),
    })
}

/// Handle SetNetworkInterfaces request.
///
/// Persists to the machine-owned overlay; anyka-init applies it at the next
/// boot. onvif-rust deliberately does not run `ifconfig` — the supervisor owns
/// the interface, and racing it is how a camera loses its only remote access.
pub async fn handle_set_network_interfaces(
    platform: &Option<Arc<dyn Platform>>,
    request: SetNetworkInterfaces,
) -> OnvifResult<SetNetworkInterfacesResponse> {
    let ipv4 = request.network_interface.ipv4.as_ref().ok_or_else(|| {
        OnvifError::invalid_arg_val("NoConfig", "IPv4 configuration block is required")
    })?;

    if !ipv4.enabled {
        return Err(unsupported_network_config("IPv4 is disabled"));
    }

    let dhcp = ipv4.dhcp;

    let (address, prefix) = if dhcp {
        (None, None)
    } else {
        let manual = ipv4.manual.first().ok_or_else(|| {
            OnvifError::invalid_arg_val("NoConfig", "static addressing requires a Manual block")
        })?;
        validate_ipv4(&manual.address)?;
        if !(1..=32).contains(&manual.prefix_length) {
            return Err(OnvifError::invalid_arg_val(
                "NoConfig",
                "PrefixLength must be between 1 and 32",
            ));
        }
        (
            Some(manual.address.clone()),
            Some(manual.prefix_length as u8),
        )
    };

    let network_info = platform
        .as_ref()
        .and_then(|p| p.network_info())
        .ok_or_else(|| OnvifError::ActionNotSupported("SetNetworkInterfaces".to_string()))?;

    network_info
        .set_network_interface(&request.interface_token, address, prefix, dhcp)
        .await
        .map_err(|e| OnvifError::HardwareFailure(e.to_string()))?;

    tracing::info!(
        dhcp,
        "SetNetworkInterfaces: persisted; applies at next boot"
    );

    Ok(SetNetworkInterfacesResponse {
        reboot_needed: true,
    })
}

/// Handle SetDNS request.
///
/// Persists to the overlay; applied by anyka-init at next boot.
pub async fn handle_set_dns(
    platform: &Option<Arc<dyn Platform>>,
    request: SetDNS,
) -> OnvifResult<SetDNSResponse> {
    let servers: Vec<String> = if request.from_dhcp {
        Vec::new()
    } else {
        let mut servers = Vec::with_capacity(request.dns_manual.len());
        for entry in &request.dns_manual {
            if entry.address_type != IPType::IPv4 {
                return Err(unsupported_network_config(
                    "IPv6 DNS servers are not supported; Type must be IPv4",
                ));
            }
            match &entry.ipv4_address {
                Some(addr) => servers.push(addr.clone()),
                None => {
                    return Err(unsupported_network_config(
                        "IPv4 DNS entries require IPv4Address",
                    ));
                }
            }
        }
        servers
    };
    for s in &servers {
        validate_ipv4(s)?;
    }

    let network_info = platform
        .as_ref()
        .and_then(|p| p.network_info())
        .ok_or_else(|| OnvifError::ActionNotSupported("SetDNS".to_string()))?;

    network_info
        .set_dns(&servers, &request.search_domain)
        .await
        .map_err(|e| OnvifError::HardwareFailure(e.to_string()))?;

    Ok(SetDNSResponse {})
}

/// Handle SetNetworkDefaultGateway request.
///
/// Persists to the overlay; applied by anyka-init at next boot.
pub async fn handle_set_network_default_gateway(
    platform: &Option<Arc<dyn Platform>>,
    request: SetNetworkDefaultGateway,
) -> OnvifResult<SetNetworkDefaultGatewayResponse> {
    let gateway = request
        .network_gateway
        .first()
        .and_then(|g| g.ipv4_address.first())
        .ok_or_else(|| OnvifError::invalid_arg_val("NoConfig", "IPv4 gateway address required"))?;
    validate_ipv4(gateway)?;

    let network_info = platform
        .as_ref()
        .and_then(|p| p.network_info())
        .ok_or_else(|| OnvifError::ActionNotSupported("SetNetworkDefaultGateway".to_string()))?;

    network_info
        .set_gateway(gateway)
        .await
        .map_err(|e| OnvifError::HardwareFailure(e.to_string()))?;

    Ok(SetNetworkDefaultGatewayResponse {})
}

/// Handle GetNTP request.
///
/// Returns NTP configuration.
pub async fn handle_get_ntp(
    platform: &Option<Arc<dyn Platform>>,
    _request: GetNTP,
) -> OnvifResult<GetNTPResponse> {
    tracing::debug!("GetNTP request");

    // Try to get NTP info from platform
    if let Some(platform) = platform
        && let Some(network_info) = platform.network_info()
        && let Ok(ntp_info) = network_info.get_ntp_info().await
    {
        // Convert platform NTP info to ONVIF NetworkHost types
        let to_network_host = |addr: &String| {
            // Check if it's an IP address or DNS name
            if let Ok(ip) = addr.parse::<std::net::IpAddr>() {
                if ip.is_ipv6() {
                    NetworkHost::ipv6(addr)
                } else {
                    NetworkHost::ipv4(addr)
                }
            } else {
                NetworkHost::dns(addr)
            }
        };

        let ntp_from_dhcp: Vec<NetworkHost> =
            ntp_info.ntp_from_dhcp.iter().map(to_network_host).collect();

        let ntp_manual: Vec<NetworkHost> =
            ntp_info.ntp_manual.iter().map(to_network_host).collect();

        return Ok(GetNTPResponse {
            ntp_information: NTPInformation {
                from_dhcp: ntp_info.from_dhcp,
                ntp_from_dhcp,
                ntp_manual,
            },
        });
    }

    // Return empty NTP info if no platform available
    Ok(GetNTPResponse {
        ntp_information: NTPInformation::default(),
    })
}

/// Handle SetNTP request.
///
/// Not supported - returns ActionNotSupported error.
pub async fn handle_set_ntp(request: SetNTP) -> OnvifResult<SetNTPResponse> {
    tracing::debug!(
        "SetNTP request: from_dhcp={}, {} manual servers (not supported)",
        request.from_dhcp,
        request.ntp_manual.len()
    );

    Err(OnvifError::ActionNotSupported("SetNTP".to_string()))
}

/// Handle GetNetworkDefaultGateway request.
///
/// Returns default gateway configuration.
pub async fn handle_get_network_default_gateway(
    config: &Arc<ConfigRuntime>,
    _request: GetNetworkDefaultGateway,
) -> OnvifResult<GetNetworkDefaultGatewayResponse> {
    tracing::debug!("GetNetworkDefaultGateway request");

    // Get gateway from config (platform doesn't expose gateway info)
    let gateway = {
        let g = config.read().network.gateway.clone();
        if g.is_empty() {
            "192.168.1.1".to_string()
        } else {
            g
        }
    };

    let network_gateway = NetworkGateway {
        ipv4_address: vec![gateway],
        ipv6_address: vec![],
        extension: None,
    };

    Ok(GetNetworkDefaultGatewayResponse {
        network_gateway: vec![network_gateway],
    })
}

/// Handle GetNetworkProtocols request.
///
/// Returns network protocol configurations.
pub async fn handle_get_network_protocols(
    platform: &Option<Arc<dyn Platform>>,
    config: &Arc<ConfigRuntime>,
    _request: GetNetworkProtocols,
) -> OnvifResult<GetNetworkProtocolsResponse> {
    tracing::debug!("GetNetworkProtocols request");

    // Try to get protocol info from platform
    if let Some(platform) = platform
        && let Some(network_info) = platform.network_info()
        && let Ok(protocols) = network_info.get_network_protocols().await
    {
        let network_protocols: Vec<NetworkProtocol> = protocols
            .iter()
            .filter_map(|p| {
                let name = match p.name.to_uppercase().as_str() {
                    "HTTP" => NetworkProtocolType::HTTP,
                    "HTTPS" => NetworkProtocolType::HTTPS,
                    "RTSP" => NetworkProtocolType::RTSP,
                    _ => return None,
                };
                Some(NetworkProtocol {
                    name,
                    enabled: p.enabled,
                    port: p.ports.iter().map(|&p| p as i32).collect(),
                })
            })
            .collect();
        return Ok(GetNetworkProtocolsResponse { network_protocols });
    }

    let cfg = config.read();
    Ok(GetNetworkProtocolsResponse {
        network_protocols: vec![
            NetworkProtocol {
                name: NetworkProtocolType::HTTP,
                enabled: true,
                port: vec![cfg.server.port as i32],
            },
            NetworkProtocol {
                name: NetworkProtocolType::RTSP,
                enabled: true,
                port: vec![cfg.media.rtsp_port as i32],
            },
        ],
    })
}

/// Handle SetNetworkProtocols request (HTTP/RTSP only; SNMP is `/api/snmp`).
pub async fn handle_set_network_protocols(
    config: &Arc<ConfigRuntime>,
    request: SetNetworkProtocols,
) -> OnvifResult<SetNetworkProtocolsResponse> {
    let (http_port, rtsp_port) = parse_http_rtsp_ports(&request)?;
    apply_http_rtsp_ports(config, http_port, rtsp_port);
    Ok(SetNetworkProtocolsResponse {})
}

fn parse_http_rtsp_ports(request: &SetNetworkProtocols) -> OnvifResult<(Option<i32>, Option<i32>)> {
    let mut http_port: Option<i32> = None;
    let mut rtsp_port: Option<i32> = None;
    for proto in &request.network_protocols {
        match proto.name {
            NetworkProtocolType::HTTPS => {
                return Err(OnvifError::ActionNotSupported(
                    "HTTPS: no TLS listener exists".to_string(),
                ));
            }
            NetworkProtocolType::HTTP | NetworkProtocolType::RTSP => {
                if !proto.enabled {
                    return Err(OnvifError::invalid_arg_val(
                        "NoConfig",
                        "disabling HTTP/RTSP listeners is not supported",
                    ));
                }
                let port = *proto.port.first().ok_or_else(|| {
                    OnvifError::invalid_arg_val("NoConfig", "protocol entry carries no port")
                })?;
                if !(1..=65535).contains(&port) {
                    return Err(OnvifError::invalid_arg_val(
                        "NoConfig",
                        "port must be between 1 and 65535",
                    ));
                }
                match proto.name {
                    NetworkProtocolType::HTTP => http_port = Some(port),
                    NetworkProtocolType::RTSP => rtsp_port = Some(port),
                    _ => unreachable!(),
                }
            }
        }
    }
    Ok((http_port, rtsp_port))
}

fn apply_http_rtsp_ports(
    config: &Arc<ConfigRuntime>,
    http_port: Option<i32>,
    rtsp_port: Option<i32>,
) {
    let mut cfg = config.write();
    if let Some(port) = http_port {
        cfg.server.port = port as u16;
    }
    if let Some(port) = rtsp_port {
        cfg.media.rtsp_port = port as u16;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigRuntime;
    use crate::onvif::types::device::{IPv4NetworkInterfaceSet, NetworkInterfaceSetConfiguration};
    use std::sync::Arc;

    fn create_test_config() -> Arc<ConfigRuntime> {
        Arc::new(ConfigRuntime::new(Default::default()))
    }

    // ========================================================================
    // GetHostname Tests (T209)
    // ========================================================================

    #[test]
    fn test_get_hostname() {
        let config = create_test_config();
        let response = handle_get_hostname(&config, GetHostname {}).unwrap();

        let info = &response.hostname_information;
        assert!(!info.from_dhcp);
        assert!(info.name.is_some());
        assert!(!info.name.as_ref().unwrap().is_empty());
    }

    // ========================================================================
    // SetHostname Tests (T210)
    // ========================================================================

    #[test]
    fn test_set_hostname_valid() {
        let config = create_test_config();
        let result = handle_set_hostname(
            &config,
            SetHostname {
                name: "my-camera".to_string(),
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_hostname_invalid_empty() {
        let config = create_test_config();
        let result = handle_set_hostname(
            &config,
            SetHostname {
                name: "".to_string(),
            },
        );
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "InvalidHostname"
        ));
    }

    #[test]
    fn test_set_hostname_invalid_chars() {
        let config = create_test_config();
        let result = handle_set_hostname(
            &config,
            SetHostname {
                name: "camera.local".to_string(),
            },
        );
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "InvalidHostname"
        ));
    }

    #[test]
    fn test_set_hostname_invalid_leading_hyphen() {
        let config = create_test_config();
        let result = handle_set_hostname(
            &config,
            SetHostname {
                name: "-camera".to_string(),
            },
        );
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "InvalidHostname"
        ));
    }

    // ========================================================================
    // GetNetworkInterfaces Tests (T211)
    // ========================================================================

    #[tokio::test]
    async fn test_get_network_interfaces_reads_platform_link_speed() {
        use crate::platform::StubPlatformBuilder;

        let config = create_test_config();
        let platform = Arc::new(
            StubPlatformBuilder::new()
                .network_info_supported(true)
                .build(),
        );
        let response =
            handle_get_network_interfaces(&Some(platform), &config, GetNetworkInterfaces {})
                .await
                .unwrap();

        let link = response.network_interfaces[0]
            .link
            .as_ref()
            .expect("link settings");
        assert_eq!(link.oper_settings.speed, 100);
        assert!(link.admin_settings.auto_negotiation);
        assert_eq!(link.admin_settings.speed, 0);
    }

    #[test]
    fn test_build_onvif_network_interface_leaves_missing_ipv4_empty() {
        let iface = crate::platform::common::NetworkInterfaceInfo {
            token: "eth1".to_string(),
            name: "eth1".to_string(),
            enabled: true,
            ipv4_address: None,
            ipv4_prefix_length: None,
            ipv4_dhcp: true,
            mac_address: Some("AA:BB:CC:DD:EE:FF".to_string()),
            link_speed: Some(1000),
        };
        let built = build_onvif_network_interface(&iface);
        let v4 = built.ipv4.expect("ipv4");
        let addr = v4
            .config
            .from_dhcp
            .as_ref()
            .map(|a| a.address.as_str())
            .unwrap_or("");
        assert_eq!(addr, "");
        let link = built.link.expect("link");
        assert!(link.admin_settings.auto_negotiation);
        assert_eq!(link.oper_settings.speed, 1000);
    }

    #[tokio::test]
    async fn test_get_network_interfaces() {
        let config = create_test_config();
        let response = handle_get_network_interfaces(&None, &config, GetNetworkInterfaces {})
            .await
            .unwrap();

        assert!(!response.network_interfaces.is_empty());
    }

    #[tokio::test]
    async fn test_get_network_interfaces_with_config() {
        let config = create_test_config();
        {
            let mut c = config.write();
            c.network.detected_ip = "192.168.1.50".to_string();
            c.network.mac_address = "AA:BB:CC:DD:EE:FF".to_string();
            c.network.dhcp_enabled = false;
        }

        let response = handle_get_network_interfaces(&None, &config, GetNetworkInterfaces {})
            .await
            .unwrap();

        let iface = &response.network_interfaces[0];
        assert_eq!(iface.token, "eth0");
    }

    // ========================================================================
    // GetDNS Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_dns() {
        let response = handle_get_dns(&None, GetDNS {}).await.unwrap();

        // Should have DNS information
        let _ = response.dns_information.from_dhcp;
    }

    // ========================================================================
    // SetNetworkInterfaces Tests
    // ========================================================================

    fn static_set_network_interfaces_request() -> SetNetworkInterfaces {
        SetNetworkInterfaces {
            interface_token: "eth0".to_string(),
            network_interface: NetworkInterfaceSetConfiguration {
                ipv4: Some(IPv4NetworkInterfaceSet {
                    enabled: true,
                    manual: vec![PrefixedIPv4Address {
                        address: "192.168.2.50".to_string(),
                        prefix_length: 24,
                    }],
                    link_local: None,
                    from_dhcp: None,
                    dhcp: false,
                }),
            },
        }
    }

    #[tokio::test]
    async fn test_set_network_interfaces_rejects_a_malformed_address() {
        let platform = None;
        let mut request = static_set_network_interfaces_request();
        request.network_interface.ipv4.as_mut().unwrap().manual[0].address =
            "not-an-ip".to_string();

        let result = handle_set_network_interfaces(&platform, request).await;

        assert!(
            matches!(result, Err(OnvifError::InvalidArgVal { .. })),
            "a malformed address must fault before reaching the overlay"
        );
    }

    #[tokio::test]
    async fn test_set_network_interfaces_rejects_static_without_an_address() {
        let platform = None;
        let request = SetNetworkInterfaces {
            interface_token: "eth0".to_string(),
            network_interface: NetworkInterfaceSetConfiguration {
                ipv4: Some(IPv4NetworkInterfaceSet {
                    enabled: true,
                    manual: vec![],
                    link_local: None,
                    from_dhcp: None,
                    dhcp: false,
                }),
            },
        };

        assert!(
            handle_set_network_interfaces(&platform, request)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_set_network_interfaces_without_a_platform_is_not_supported() {
        let platform = None;
        let request = SetNetworkInterfaces {
            interface_token: "eth0".to_string(),
            network_interface: NetworkInterfaceSetConfiguration {
                ipv4: Some(IPv4NetworkInterfaceSet {
                    enabled: true,
                    manual: vec![],
                    link_local: None,
                    from_dhcp: None,
                    dhcp: true,
                }),
            },
        };

        assert!(matches!(
            handle_set_network_interfaces(&platform, request).await,
            Err(OnvifError::ActionNotSupported(_))
        ));
    }

    // ========================================================================
    // SetDNS Tests
    // ========================================================================

    #[tokio::test]
    async fn test_set_dns_with_manual_servers_succeeds() {
        use crate::platform::StubPlatformBuilder;

        let platform = Arc::new(
            StubPlatformBuilder::new()
                .network_info_supported(true)
                .build(),
        );
        let request = SetDNS {
            from_dhcp: false,
            search_domain: vec![],
            dns_manual: vec![IPAddress::ipv4("8.8.8.8")],
        };

        let result = handle_set_dns(&Some(platform), request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_dns_rejects_ipv6_only_entries() {
        let platform = None;
        let request = SetDNS {
            from_dhcp: false,
            search_domain: vec![],
            dns_manual: vec![IPAddress::ipv6("2001:db8::1")],
        };

        assert!(matches!(
            handle_set_dns(&platform, request).await,
            Err(OnvifError::InvalidArgVal { .. })
        ));
    }

    #[tokio::test]
    async fn test_set_dns_rejects_ipv6_typed_entry_with_ipv4_address() {
        let platform = None;
        let request = SetDNS {
            from_dhcp: false,
            search_domain: vec![],
            dns_manual: vec![IPAddress {
                address_type: IPType::IPv6,
                ipv4_address: Some("8.8.8.8".to_string()),
                ipv6_address: None,
            }],
        };

        assert!(matches!(
            handle_set_dns(&platform, request).await,
            Err(OnvifError::InvalidArgVal { .. })
        ));
    }

    #[tokio::test]
    async fn test_set_dns_rejects_a_malformed_server() {
        let platform = None;
        let request = SetDNS {
            from_dhcp: false,
            search_domain: vec![],
            dns_manual: vec![IPAddress::ipv4("999.1.1.1")],
        };

        assert!(matches!(
            handle_set_dns(&platform, request).await,
            Err(OnvifError::InvalidArgVal { .. })
        ));
    }

    #[tokio::test]
    async fn test_set_dns_from_dhcp_clears_manual_servers() {
        use crate::platform::StubPlatformBuilder;

        let platform = Arc::new(
            StubPlatformBuilder::new()
                .network_info_supported(true)
                .build(),
        );
        let request = SetDNS {
            from_dhcp: true,
            search_domain: vec![],
            dns_manual: vec![IPAddress::ipv4("8.8.8.8")],
        };

        handle_set_dns(&Some(platform.clone()), request)
            .await
            .expect("must succeed");

        let dns = platform
            .network_info()
            .expect("network info")
            .get_dns_info()
            .await
            .expect("dns");
        assert!(dns.dns_manual.is_empty());
    }

    // ========================================================================
    // GetNTP Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_ntp() {
        let response = handle_get_ntp(&None, GetNTP {}).await.unwrap();

        // Should have NTP information
        let _ = response.ntp_information.from_dhcp;
    }

    // ========================================================================
    // SetNTP Tests
    // ========================================================================

    #[tokio::test]
    async fn test_set_ntp_not_supported() {
        let result = handle_set_ntp(SetNTP {
            from_dhcp: false,
            ntp_manual: vec![NetworkHost::ipv4("pool.ntp.org")],
        })
        .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    // ========================================================================
    // GetNetworkDefaultGateway Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_network_default_gateway() {
        let config = create_test_config();
        let response = handle_get_network_default_gateway(&config, GetNetworkDefaultGateway {})
            .await
            .unwrap();

        // Should have gateway information
        assert!(!response.network_gateway.is_empty());
    }

    // ========================================================================
    // GetNetworkProtocols Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_network_protocols() {
        let config = create_test_config();
        let response = handle_get_network_protocols(&None, &config, GetNetworkProtocols {})
            .await
            .unwrap();

        assert!(!response.network_protocols.is_empty());
        assert!(
            response
                .network_protocols
                .iter()
                .any(|p| p.name == NetworkProtocolType::HTTP),
            "HTTP must be advertised"
        );
        assert!(
            response
                .network_protocols
                .iter()
                .any(|p| p.name == NetworkProtocolType::RTSP),
            "RTSP must be advertised"
        );
    }

    #[tokio::test]
    async fn test_set_network_protocols_updates_http_and_rtsp_ports() {
        let config = create_test_config();
        let request = SetNetworkProtocols {
            network_protocols: vec![
                NetworkProtocol {
                    name: NetworkProtocolType::HTTP,
                    enabled: true,
                    port: vec![8080],
                },
                NetworkProtocol {
                    name: NetworkProtocolType::RTSP,
                    enabled: true,
                    port: vec![8554],
                },
            ],
        };

        handle_set_network_protocols(&config, request)
            .await
            .expect("must succeed");

        assert_eq!(config.read().server.port, 8080);
        assert_eq!(config.read().media.rtsp_port, 8554);
    }

    #[tokio::test]
    async fn test_set_network_protocols_rejects_https() {
        let config = create_test_config();
        let request = SetNetworkProtocols {
            network_protocols: vec![NetworkProtocol {
                name: NetworkProtocolType::HTTPS,
                enabled: true,
                port: vec![443],
            }],
        };

        assert!(
            handle_set_network_protocols(&config, request)
                .await
                .is_err(),
            "there is no TLS listener"
        );
    }

    #[tokio::test]
    async fn test_set_network_protocols_rejects_disabled_listener() {
        let config = create_test_config();
        let before = config.read().server.port;
        let request = SetNetworkProtocols {
            network_protocols: vec![NetworkProtocol {
                name: NetworkProtocolType::HTTP,
                enabled: false,
                port: vec![8080],
            }],
        };
        assert!(
            handle_set_network_protocols(&config, request)
                .await
                .is_err()
        );
        assert_eq!(config.read().server.port, before);
    }

    #[tokio::test]
    async fn test_set_network_protocols_rejects_port_zero() {
        let config = create_test_config();
        let request = SetNetworkProtocols {
            network_protocols: vec![NetworkProtocol {
                name: NetworkProtocolType::HTTP,
                enabled: true,
                port: vec![0],
            }],
        };
        assert!(
            handle_set_network_protocols(&config, request)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_set_network_protocols_leaves_config_unchanged_on_rejected_request() {
        let config = create_test_config();
        let before_http = config.read().server.port;
        let before_rtsp = config.read().media.rtsp_port;
        let request = SetNetworkProtocols {
            network_protocols: vec![
                NetworkProtocol {
                    name: NetworkProtocolType::HTTP,
                    enabled: true,
                    port: vec![8080],
                },
                NetworkProtocol {
                    name: NetworkProtocolType::HTTPS,
                    enabled: true,
                    port: vec![443],
                },
            ],
        };
        assert!(
            handle_set_network_protocols(&config, request)
                .await
                .is_err()
        );
        assert_eq!(config.read().server.port, before_http);
        assert_eq!(config.read().media.rtsp_port, before_rtsp);
    }

    #[tokio::test]
    async fn test_set_network_interfaces_rejects_missing_ipv4_block() {
        let platform = None;
        let request = SetNetworkInterfaces {
            interface_token: "eth0".to_string(),
            network_interface: NetworkInterfaceSetConfiguration { ipv4: None },
        };
        assert!(
            handle_set_network_interfaces(&platform, request)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_set_network_interfaces_rejects_disabled_ipv4() {
        let platform = None;
        let request = SetNetworkInterfaces {
            interface_token: "eth0".to_string(),
            network_interface: NetworkInterfaceSetConfiguration {
                ipv4: Some(IPv4NetworkInterfaceSet {
                    enabled: false,
                    manual: vec![],
                    link_local: None,
                    from_dhcp: None,
                    dhcp: true,
                }),
            },
        };
        assert!(
            handle_set_network_interfaces(&platform, request)
                .await
                .is_err()
        );
    }

    #[test]
    fn test_set_network_interfaces_wire_shape_for_dhcp_and_manual() {
        let dhcp_ipv4 = IPv4NetworkInterfaceSet {
            enabled: true,
            dhcp: true,
            ..Default::default()
        };
        let dhcp_xml = quick_xml::se::to_string(&dhcp_ipv4).expect("serialize dhcp ipv4");
        assert!(
            dhcp_xml.contains("tt:DHCP") || dhcp_xml.contains("<tt:DHCP>"),
            "expected flat tt:DHCP under tt:IPv4, got: {dhcp_xml}"
        );

        let manual_ipv4 = IPv4NetworkInterfaceSet {
            enabled: true,
            manual: vec![PrefixedIPv4Address {
                address: "192.168.2.50".to_string(),
                prefix_length: 24,
            }],
            dhcp: false,
            ..Default::default()
        };
        let manual_xml = quick_xml::se::to_string(&manual_ipv4).expect("serialize manual ipv4");
        assert!(
            manual_xml.contains("tt:Manual") || manual_xml.contains("<tt:Manual>"),
            "expected tt:Manual under tt:IPv4, got: {manual_xml}"
        );
    }
}
