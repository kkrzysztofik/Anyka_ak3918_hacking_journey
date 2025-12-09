use onvif_rust::onvif::types::common::*;
use onvif_rust::onvif::types::device::*;
use quick_xml::se::to_string;

#[test]
fn test_get_capabilities_response_serialization_prefixes() {
    let response = GetCapabilitiesResponse {
        capabilities: Capabilities {
            analytics: Some(AnalyticsCapabilities {
                x_addr: "http://host/analytics".to_string(),
                rule_support: true,
                analytics_module_support: true,
            }),
            device: Some(DeviceCapabilities {
                x_addr: "http://host/device".to_string(),
                network: Some(NetworkCapabilitiesLegacy {
                    ip_filter: Some(true),
                    zero_configuration: Some(true),
                    ip_version6: Some(true),
                    dyn_dns: Some(true),
                    extension: Some(NetworkCapabilitiesExtension {
                        dot11_configuration: Some(true),
                    }),
                }),
                system: Some(SystemCapabilitiesLegacy {
                    discovery_resolve: true,
                    discovery_bye: true,
                    remote_discovery: true,
                    system_backup: true,
                    system_logging: Some(true),
                    firmware_upgrade: true,
                    supported_versions: vec![OnvifVersion { major: 2, minor: 5 }],
                    extension: Some(SystemCapabilitiesExtension {
                        http_firmware_upgrade: Some(true),
                        http_system_backup: Some(true),
                        http_system_logging: Some(true),
                        http_support_information: Some(true),
                        extension: None,
                    }),
                }),
                io: Some(IOCapabilities {
                    input_connectors: Some(1),
                    relay_outputs: Some(1),
                    extension: Some(IOCapabilitiesExtension {
                        auxiliary: Some(true),
                        extension: None,
                    }),
                }),
                security: Some(SecurityCapabilitiesLegacy {
                    tls1_1: true,
                    tls1_2: true,
                    onboard_key_generation: true,
                    access_policy_config: true,
                    x509_token: true,
                    saml_token: true,
                    kerberos_token: true,
                    rel_token: true,
                    extension: Some(SecurityCapabilitiesExtension {
                        tls1_0: Some(true),
                        extension: Some(SecurityCapabilitiesExtension2 {
                            dot1x: Some(true),
                            supported_eap_method: Some(1),
                            remote_user_handling: Some(true),
                        }),
                    }),
                }),
                extension: None,
            }),
            events: Some(EventsCapabilities {
                x_addr: "http://host/events".to_string(),
                ws_subscription_policy_support: true,
                ws_pull_point_support: true,
                ws_pausable_subscription_manager_interface_support: true,
            }),
            imaging: Some(ImagingCapabilities {
                x_addr: "http://host/imaging".to_string(),
            }),
            media: Some(MediaCapabilities {
                x_addr: "http://host/media".to_string(),
                streaming_capabilities: Some(RealTimeStreamingCapabilities {
                    rtp_multicast: Some(true),
                    rtp_tcp: Some(true),
                    rtp_rtsp_tcp: Some(true),
                    extension: None,
                }),
                extension: Some(MediaCapabilitiesExtension {
                    profile_capabilities: Some(ProfileCapabilities {
                        maximum_number_of_profiles: Some(10),
                    }),
                }),
            }),
            ptz: Some(PTZCapabilities {
                x_addr: "http://host/ptz".to_string(),
            }),
            extension: Some(CapabilitiesExtension {
                device_io: Some(DeviceIOCapabilities {
                    x_addr: "http://host/deviceio".to_string(),
                    video_sources: Some(1),
                    video_outputs: Some(1),
                    audio_sources: Some(1),
                    audio_outputs: Some(1),
                    relay_outputs: Some(1),
                }),
            }),
        },
    };

    let xml = to_string(&response).expect("Failed to serialize GetCapabilitiesResponse");

    // Verify top-level
    assert!(xml.contains("<tds:GetCapabilitiesResponse>"));
    assert!(xml.contains("<tds:Capabilities>"));

    // Verify tt: prefixes for major sections
    assert!(xml.contains("<tt:Analytics>"));
    assert!(xml.contains("<tt:Device>"));
    assert!(xml.contains("<tt:Events>"));
    assert!(xml.contains("<tt:Imaging>"));
    assert!(xml.contains("<tt:Media>"));
    assert!(xml.contains("<tt:PTZ>"));
    assert!(xml.contains("<tt:Extension>"));

    // Verify nested elements have tt: prefix
    assert!(xml.contains("<tt:XAddr>http://host/analytics</tt:XAddr>"));
    assert!(xml.contains("<tt:RuleSupport>true</tt:RuleSupport>"));
    assert!(xml.contains("<tt:Network>"));
    assert!(xml.contains("<tt:System>"));
    assert!(xml.contains("<tt:IO>"));
    assert!(xml.contains("<tt:Security>"));

    // Verify deep nested elements
    assert!(xml.contains("<tt:IPFilter>true</tt:IPFilter>"));
    assert!(xml.contains("<tt:DiscoveryResolve>true</tt:DiscoveryResolve>"));
    assert!(xml.contains("<tt:InputConnectors>1</tt:InputConnectors>"));
    assert!(xml.contains("<tt:TLS1.1>true</tt:TLS1.1>"));
    assert!(xml.contains("<tt:StreamingCapabilities>"));
    assert!(xml.contains("<tt:RTPMulticast>true</tt:RTPMulticast>"));
    assert!(xml.contains("<tt:ProfileCapabilities>"));
    assert!(xml.contains("<tt:MaximumNumberOfProfiles>10</tt:MaximumNumberOfProfiles>"));
    assert!(xml.contains("<tt:DeviceIO>"));
    assert!(xml.contains("<tt:VideoSources>1</tt:VideoSources>"));
}

#[test]
fn test_get_network_interfaces_response_serialization_prefixes() {
    let response = GetNetworkInterfacesResponse {
        network_interfaces: vec![NetworkInterface {
            token: "eth0".to_string(),
            enabled: true,
            info: Some(NetworkInterfaceInfo {
                name: Some("eth0".to_string()),
                hw_address: "00:11:22:33:44:55".to_string(),
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
                interface_type: 6,
            }),
            ipv4: Some(IPv4NetworkInterface {
                enabled: true,
                config: IPv4Configuration {
                    manual: vec![PrefixedIPv4Address {
                        address: "192.168.1.10".to_string(),
                        prefix_length: 24,
                    }],
                    link_local: None,
                    from_dhcp: None,
                    dhcp: false,
                },
            }),
            ipv6: None,
            extension: None,
        }],
    };

    let xml = to_string(&response).expect("Failed to serialize GetNetworkInterfacesResponse");

    assert!(xml.contains("<tds:GetNetworkInterfacesResponse>"));
    assert!(xml.contains("<tds:NetworkInterfaces token=\"eth0\">"));

    // Check for tt: prefixes
    assert!(xml.contains("<tt:Enabled>true</tt:Enabled>"));
    assert!(xml.contains("<tt:Info>"));
    assert!(xml.contains("<tt:Name>eth0</tt:Name>"));
    assert!(xml.contains("<tt:HwAddress>00:11:22:33:44:55</tt:HwAddress>"));
    assert!(xml.contains("<tt:Link>"));
    assert!(xml.contains("<tt:AdminSettings>"));
    assert!(xml.contains("<tt:AutoNegotiation>true</tt:AutoNegotiation>"));
    assert!(xml.contains("<tt:IPv4>"));
    assert!(xml.contains("<tt:Config>"));
    assert!(xml.contains("<tt:Manual>"));
    assert!(xml.contains("<tt:Address>192.168.1.10</tt:Address>"));
}

#[test]
fn test_get_dns_response_serialization_prefixes() {
    let response = GetDNSResponse {
        dns_information: DNSInformation {
            from_dhcp: false,
            search_domain: vec!["example.com".to_string()],
            dns_from_dhcp: vec![],
            dns_manual: vec![IPAddress {
                address_type: IPType::IPv4,
                ipv4_address: Some("8.8.8.8".to_string()),
                ipv6_address: None,
            }],
        },
    };

    let xml = to_string(&response).expect("Failed to serialize GetDNSResponse");

    assert!(xml.contains("<tds:GetDNSResponse>"));
    assert!(xml.contains("<tds:DNSInformation>"));

    // Check for tt: prefixes
    assert!(xml.contains("<tt:FromDHCP>false</tt:FromDHCP>"));
    assert!(xml.contains("<tt:SearchDomain>example.com</tt:SearchDomain>"));
    assert!(xml.contains("<tt:DNSManual>"));
    assert!(xml.contains("<tt:Type>IPv4</tt:Type>"));
    assert!(xml.contains("<tt:IPv4Address>8.8.8.8</tt:IPv4Address>"));
}
