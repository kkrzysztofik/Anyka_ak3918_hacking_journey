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
use crate::onvif::device::faults::validate_hostname;
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::device::{
    DNSInformation, Duplex, GetDNS, GetDNSResponse, GetHostname, GetHostnameResponse, GetNTP,
    GetNTPResponse, GetNetworkDefaultGateway, GetNetworkDefaultGatewayResponse,
    GetNetworkInterfaces, GetNetworkInterfacesResponse, GetNetworkProtocols,
    GetNetworkProtocolsResponse, HostnameInformation, IPAddress, IPv4Configuration,
    IPv4NetworkInterface, NTPInformation, NetworkGateway, NetworkHost, NetworkInterface,
    NetworkInterfaceConnectionSetting, NetworkInterfaceInfo, NetworkInterfaceLink, NetworkProtocol,
    NetworkProtocolType, PrefixedIPv4Address, SetDNS, SetDNSResponse, SetHostname,
    SetHostnameResponse, SetNTP, SetNTPResponse, SetNetworkProtocols, SetNetworkProtocolsResponse,
};
use crate::platform::{Platform, external_ip};

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

    let (ip_address, mac_address, dhcp_enabled) = get_network_info(platform, config).await;

    // Create network interface
    let network_interface = NetworkInterface {
        token: "eth0".to_string(),
        enabled: true,
        info: Some(NetworkInterfaceInfo {
            name: Some("eth0".to_string()),
            hw_address: mac_address,
            mtu: Some(1500),
        }),
        link: Some(NetworkInterfaceLink {
            admin_settings: NetworkInterfaceConnectionSetting {
                auto_negotiation: true,
                speed: 100,
                duplex: Duplex::Full,
            },
            oper_settings: NetworkInterfaceConnectionSetting {
                auto_negotiation: true,
                speed: 100,
                duplex: Duplex::Full,
            },
            interface_type: 6, // ethernetCsmacd
        }),
        // TODO: Read actual prefix length from network interface or config.
        // /24 is a reasonable default for the embedded camera's typical LAN deployment.
        ipv4: Some(IPv4NetworkInterface {
            enabled: true,
            config: IPv4Configuration {
                manual: if !dhcp_enabled {
                    vec![PrefixedIPv4Address {
                        address: ip_address.clone(),
                        prefix_length: 24,
                    }]
                } else {
                    vec![]
                },
                link_local: None,
                from_dhcp: if dhcp_enabled {
                    Some(PrefixedIPv4Address {
                        address: ip_address,
                        prefix_length: 24,
                    })
                } else {
                    None
                },
                dhcp: dhcp_enabled,
            },
        }),
        ipv6: None,
        extension: None,
    };

    Ok(GetNetworkInterfacesResponse {
        network_interfaces: vec![network_interface],
    })
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
            .map(|addr| IPAddress::ipv4(addr))
            .collect();

        let dns_manual: Vec<IPAddress> = dns_info
            .dns_manual
            .iter()
            .map(|addr| IPAddress::ipv4(addr))
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

/// Handle SetDNS request.
///
/// Not supported - returns ActionNotSupported error.
pub async fn handle_set_dns(request: SetDNS) -> OnvifResult<SetDNSResponse> {
    tracing::debug!(
        "SetDNS request: from_dhcp={}, {} manual servers (not supported)",
        request.from_dhcp,
        request.dns_manual.len()
    );

    Err(OnvifError::ActionNotSupported("SetDNS".to_string()))
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
            if addr.parse::<std::net::IpAddr>().is_ok() {
                NetworkHost::ipv4(addr)
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
        // Convert platform protocol info to ONVIF types
        let network_protocols: Vec<NetworkProtocol> = protocols
            .iter()
            .map(|p| {
                let name = match p.name.to_uppercase().as_str() {
                    "HTTP" => NetworkProtocolType::HTTP,
                    "HTTPS" => NetworkProtocolType::HTTPS,
                    "RTSP" => NetworkProtocolType::RTSP,
                    _ => NetworkProtocolType::HTTP, // Default fallback
                };
                NetworkProtocol {
                    name,
                    enabled: p.enabled,
                    port: p.ports.iter().map(|&p| p as i32).collect(),
                }
            })
            .collect();

        return Ok(GetNetworkProtocolsResponse { network_protocols });
    }

    // Return default protocol info from config if no platform
    let http_port = config.read().server.port as i32;

    Ok(GetNetworkProtocolsResponse {
        network_protocols: vec![
            NetworkProtocol {
                name: NetworkProtocolType::HTTP,
                enabled: true,
                port: vec![http_port],
            },
            NetworkProtocol {
                name: NetworkProtocolType::RTSP,
                enabled: true,
                port: vec![554],
            },
        ],
    })
}

/// Handle SetNetworkProtocols request.
///
/// Not supported - returns ActionNotSupported error.
pub async fn handle_set_network_protocols(
    request: SetNetworkProtocols,
) -> OnvifResult<SetNetworkProtocolsResponse> {
    tracing::debug!(
        "SetNetworkProtocols request: {} protocols (not supported)",
        request.network_protocols.len()
    );

    Err(OnvifError::ActionNotSupported(
        "SetNetworkProtocols".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigRuntime;
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
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:InvalidHostname"
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
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:InvalidHostname"
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
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:InvalidHostname"
        ));
    }

    // ========================================================================
    // GetNetworkInterfaces Tests (T211)
    // ========================================================================

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
    // SetDNS Tests
    // ========================================================================

    #[tokio::test]
    async fn test_set_dns_not_supported() {
        let result = handle_set_dns(SetDNS {
            from_dhcp: false,
            search_domain: vec!["example.com".to_string()],
            dns_manual: vec![IPAddress::ipv4("8.8.8.8")],
        })
        .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
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

        // Should have at least HTTP protocol
        assert!(!response.network_protocols.is_empty());
    }

    #[tokio::test]
    async fn test_set_network_protocols_not_supported() {
        let result = handle_set_network_protocols(SetNetworkProtocols {
            network_protocols: vec![NetworkProtocol {
                name: NetworkProtocolType::HTTP,
                enabled: true,
                port: vec![8080],
            }],
        })
        .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }
}
