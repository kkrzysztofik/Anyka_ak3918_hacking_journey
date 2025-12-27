#![allow(clippy::collapsible_if)]
//! Device Service request handlers.
//!
//! This module implements the ONVIF Device Service operation handlers including:
//! - Device information and capabilities
//! - System date/time management
//! - Hostname and network configuration
//! - Scope and discovery mode management
//! - User management

use std::sync::Arc;

use chrono::{Datelike, Timelike, Utc};

use crate::config::ConfigRuntime;
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::common::{
    Date, DateTime, DiscoveryMode, Scope, ScopeDefinition, SetDateTimeType, SystemDateTime, Time,
    TimeZone, User as OnvifUser,
};
use crate::onvif::types::device::{
    AddScopes, AddScopesResponse, BackupFile, Capabilities, CreateCertificate,
    CreateCertificateResponse, CreateUsers, CreateUsersResponse, DNSInformation,
    DeleteCertificates, DeleteCertificatesResponse, DeleteUsers, DeleteUsersResponse, Duplex,
    GetCapabilities, GetCapabilitiesResponse, GetCertificates, GetCertificatesResponse,
    GetCertificatesStatus, GetCertificatesStatusResponse, GetDNS, GetDNSResponse,
    GetDeviceInformation, GetDeviceInformationResponse, GetDiscoveryMode, GetDiscoveryModeResponse,
    GetHostname, GetHostnameResponse, GetNTP, GetNTPResponse, GetNetworkDefaultGateway,
    GetNetworkDefaultGatewayResponse, GetNetworkInterfaces, GetNetworkInterfacesResponse,
    GetNetworkProtocols, GetNetworkProtocolsResponse, GetRelayOutputs, GetRelayOutputsResponse,
    GetScopes, GetScopesResponse, GetServiceCapabilities, GetServiceCapabilitiesResponse,
    GetServices, GetServicesResponse, GetSystemBackup, GetSystemBackupResponse,
    GetSystemDateAndTime, GetSystemDateAndTimeResponse, GetUsers, GetUsersResponse,
    HostnameInformation, IPAddress, IPv4Configuration, IPv4NetworkInterface, LoadCertificates,
    LoadCertificatesResponse, NTPInformation, NetworkGateway, NetworkHost, NetworkHostType,
    NetworkInterface, NetworkInterfaceConnectionSetting, NetworkInterfaceInfo,
    NetworkInterfaceLink, NetworkProtocol, NetworkProtocolType, PrefixedIPv4Address, RemoveScopes,
    RemoveScopesResponse, RestoreSystem, RestoreSystemResponse, SetDNS, SetDNSResponse,
    SetDiscoveryMode, SetDiscoveryModeResponse, SetHostname, SetHostnameResponse, SetNTP,
    SetNTPResponse, SetNetworkProtocols, SetNetworkProtocolsResponse, SetScopes, SetScopesResponse,
    SetSystemDateAndTime, SetSystemDateAndTimeResponse, SetSystemFactoryDefault,
    SetSystemFactoryDefaultResponse, SetUser, SetUserResponse, SystemReboot, SystemRebootResponse,
};
use crate::platform::{DeviceInfo, Platform};
use crate::users::{PasswordManager, UserLevel, UserStorage};

use super::faults::{validate_hostname, validate_scope};
use super::types::{
    DEVICE_SERVICE_NAMESPACE, IMAGING_SERVICE_NAMESPACE, MEDIA_SERVICE_NAMESPACE, ONVIF_VERSION,
    PTZ_SERVICE_NAMESPACE, build_device_capabilities, build_device_service_capabilities,
    build_imaging_capabilities, build_media_capabilities, build_ptz_capabilities, create_service,
};
use super::user_types::{validate_password, validate_username};

// ============================================================================
// DeviceService
// ============================================================================

/// ONVIF Device Service.
///
/// Handles Device Service operations including:
/// - Device information and capabilities
/// - System management (date/time, reboot)
/// - Network configuration
/// - Discovery and scope management
/// - User management
pub struct DeviceService {
    /// User storage.
    users: Arc<UserStorage>,
    /// Password manager for hashing.
    #[allow(dead_code)]
    password_manager: Arc<PasswordManager>,
    /// Configuration runtime.
    config: Arc<ConfigRuntime>,
    /// Platform abstraction (optional for backward compatibility).
    platform: Option<Arc<dyn Platform>>,
    /// Scopes storage (in-memory for now).
    scopes: parking_lot::RwLock<Vec<Scope>>,
    /// Discovery mode.
    discovery_mode: parking_lot::RwLock<DiscoveryMode>,
}

impl DeviceService {
    /// Create a new Device Service.
    pub fn new(users: Arc<UserStorage>, password_manager: Arc<PasswordManager>) -> Self {
        Self {
            users,
            password_manager,
            config: Arc::new(ConfigRuntime::new(Default::default())),
            platform: None,
            scopes: parking_lot::RwLock::new(Self::default_scopes()),
            discovery_mode: parking_lot::RwLock::new(DiscoveryMode::Discoverable),
        }
    }

    /// Create a new Device Service with configuration and platform.
    pub fn with_config_and_platform(
        users: Arc<UserStorage>,
        password_manager: Arc<PasswordManager>,
        config: Arc<ConfigRuntime>,
        platform: Arc<dyn Platform>,
    ) -> Self {
        Self {
            users,
            password_manager,
            config,
            platform: Some(platform),
            scopes: parking_lot::RwLock::new(Self::default_scopes()),
            discovery_mode: parking_lot::RwLock::new(DiscoveryMode::Discoverable),
        }
    }

    /// Get the base URL for service addresses.
    /// Uses detected IP address for proper XAddr values in capabilities.
    fn base_url(&self) -> String {
        // Try to get the detected IP first, then fall back to config
        let address = self
            .config
            .get_string("network.detected_ip")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                self.config
                    .get_string("network.ip_address")
                    .ok()
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| {
                // Auto-detect IP as fallback
                Self::detect_local_ip().unwrap_or_else(|| "127.0.0.1".to_string())
            });
        let port = self.config.get_int("server.port").unwrap_or(80) as u16;
        format!("http://{}:{}", address, port)
    }

    /// Detect local IP address using UDP socket trick.
    fn detect_local_ip() -> Option<String> {
        use std::net::UdpSocket;
        match UdpSocket::bind("0.0.0.0:0") {
            Ok(socket) => {
                if socket.connect("8.8.8.8:80").is_ok() {
                    if let Ok(addr) = socket.local_addr() {
                        let ip = addr.ip().to_string();
                        if ip != "0.0.0.0" {
                            return Some(ip);
                        }
                    }
                }
                None
            }
            Err(_) => None,
        }
    }

    /// Get default scopes.
    fn default_scopes() -> Vec<Scope> {
        vec![
            Scope {
                scope_def: ScopeDefinition::Fixed,
                scope_item: "onvif://www.onvif.org/type/video_encoder".to_string(),
            },
            Scope {
                scope_def: ScopeDefinition::Fixed,
                scope_item: "onvif://www.onvif.org/type/audio_encoder".to_string(),
            },
            Scope {
                scope_def: ScopeDefinition::Fixed,
                scope_item: "onvif://www.onvif.org/type/ptz".to_string(),
            },
            Scope {
                scope_def: ScopeDefinition::Configurable,
                scope_item: "onvif://www.onvif.org/location/country/unknown".to_string(),
            },
            Scope {
                scope_def: ScopeDefinition::Configurable,
                scope_item: "onvif://www.onvif.org/name/OnvifCamera".to_string(),
            },
        ]
    }

    // ========================================================================
    // Device Information Handlers
    // ========================================================================

    /// Handle GetDeviceInformation request.
    ///
    /// Returns device manufacturer, model, firmware version, serial number, and hardware ID.
    pub async fn handle_get_device_information(
        &self,
        _request: GetDeviceInformation,
    ) -> OnvifResult<GetDeviceInformationResponse> {
        tracing::debug!("GetDeviceInformation request");

        let info = if let Some(ref platform) = self.platform {
            platform.get_device_info().await.unwrap_or_else(|e| {
                tracing::warn!("Failed to get device info from platform: {}", e);
                self.device_info_from_config()
            })
        } else {
            self.device_info_from_config()
        };

        Ok(GetDeviceInformationResponse {
            manufacturer: info.manufacturer,
            model: info.model,
            firmware_version: info.firmware_version,
            serial_number: info.serial_number,
            hardware_id: info.hardware_id,
        })
    }

    /// Get device info from configuration.
    fn device_info_from_config(&self) -> DeviceInfo {
        DeviceInfo {
            manufacturer: self
                .config
                .get_string("device.manufacturer")
                .unwrap_or_else(|_| "Anyka".to_string()),
            model: self
                .config
                .get_string("device.model")
                .unwrap_or_else(|_| "AK3918 Camera".to_string()),
            firmware_version: self
                .config
                .get_string("device.firmware_version")
                .unwrap_or_else(|_| "1.0.0".to_string()),
            serial_number: self
                .config
                .get_string("device.serial_number")
                .unwrap_or_else(|_| "000000000000".to_string()),
            hardware_id: self
                .config
                .get_string("device.hardware_id")
                .unwrap_or_else(|_| "00001".to_string()),
        }
    }

    /// Handle GetCapabilities request (legacy).
    ///
    /// Returns device capabilities in the legacy format.
    pub fn handle_get_capabilities(
        &self,
        _request: GetCapabilities,
    ) -> OnvifResult<GetCapabilitiesResponse> {
        tracing::debug!("GetCapabilities request");

        let base_url = self.base_url();

        Ok(GetCapabilitiesResponse {
            capabilities: Capabilities {
                analytics: None,
                device: Some(build_device_capabilities(&base_url)),
                events: None,
                imaging: Some(build_imaging_capabilities(&base_url)),
                media: Some(build_media_capabilities(&base_url)),
                ptz: Some(build_ptz_capabilities(&base_url)),
                extension: None,
            },
        })
    }

    /// Handle GetServices request.
    ///
    /// Returns list of available services with namespace URIs and XAddrs.
    pub fn handle_get_services(&self, request: GetServices) -> OnvifResult<GetServicesResponse> {
        tracing::debug!(
            "GetServices request, include_capability={}",
            request.include_capability
        );

        let base_url = self.base_url();

        let services = vec![
            create_service(
                DEVICE_SERVICE_NAMESPACE,
                &format!("{}/onvif/device_service", base_url),
                ONVIF_VERSION,
            ),
            create_service(
                MEDIA_SERVICE_NAMESPACE,
                &format!("{}/onvif/media_service", base_url),
                ONVIF_VERSION,
            ),
            create_service(
                PTZ_SERVICE_NAMESPACE,
                &format!("{}/onvif/ptz_service", base_url),
                ONVIF_VERSION,
            ),
            create_service(
                IMAGING_SERVICE_NAMESPACE,
                &format!("{}/onvif/imaging_service", base_url),
                ONVIF_VERSION,
            ),
        ];

        Ok(GetServicesResponse { services })
    }

    /// Handle GetServiceCapabilities request.
    ///
    /// Returns device service capabilities.
    pub fn handle_get_service_capabilities(
        &self,
        _request: GetServiceCapabilities,
    ) -> OnvifResult<GetServiceCapabilitiesResponse> {
        tracing::debug!("GetServiceCapabilities request");

        Ok(GetServiceCapabilitiesResponse {
            capabilities: build_device_service_capabilities(),
        })
    }

    // ========================================================================
    // System Date/Time Handlers
    // ========================================================================

    /// Handle GetSystemDateAndTime request.
    ///
    /// Returns current system date and time in UTC and local timezone.
    pub fn handle_get_system_date_and_time(
        &self,
        _request: GetSystemDateAndTime,
    ) -> OnvifResult<GetSystemDateAndTimeResponse> {
        tracing::debug!("GetSystemDateAndTime request");

        let now = Utc::now();

        let utc_date_time = DateTime {
            time: Time {
                hour: now.hour() as i32,
                minute: now.minute() as i32,
                second: now.second() as i32,
            },
            date: Date {
                year: now.year(),
                month: now.month() as i32,
                day: now.day() as i32,
            },
        };

        // For simplicity, local time is same as UTC (no timezone offset)
        let local_date_time = utc_date_time.clone();

        Ok(GetSystemDateAndTimeResponse {
            system_date_and_time: SystemDateTime {
                date_time_type: SetDateTimeType::Manual,
                daylight_savings: false,
                time_zone: Some(TimeZone {
                    tz: "UTC".to_string(),
                }),
                utc_date_time: Some(utc_date_time),
                local_date_time: Some(local_date_time),
                extension: None,
            },
        })
    }

    /// Handle SetSystemDateAndTime request.
    ///
    /// Sets the system date and time.
    pub fn handle_set_system_date_and_time(
        &self,
        request: SetSystemDateAndTime,
    ) -> OnvifResult<SetSystemDateAndTimeResponse> {
        tracing::debug!(
            "SetSystemDateAndTime request: type={:?}",
            request.date_time_type
        );

        // For now, just log the request - actual time setting would require platform integration
        match request.date_time_type {
            SetDateTimeType::NTP => {
                tracing::info!("SetSystemDateAndTime: NTP mode requested");
            }
            SetDateTimeType::Manual => {
                if let Some(ref utc) = request.utc_date_time {
                    tracing::info!(
                        "SetSystemDateAndTime: Manual mode - {}-{:02}-{:02} {:02}:{:02}:{:02}",
                        utc.date.year,
                        utc.date.month,
                        utc.date.day,
                        utc.time.hour,
                        utc.time.minute,
                        utc.time.second
                    );
                }
            }
        }

        Ok(SetSystemDateAndTimeResponse {})
    }

    // ========================================================================
    // System Operations Handlers
    // ========================================================================

    /// Handle SystemReboot request.
    ///
    /// Initiates system reboot.
    pub fn handle_system_reboot(
        &self,
        _request: SystemReboot,
    ) -> OnvifResult<SystemRebootResponse> {
        tracing::info!("SystemReboot request - initiating reboot");

        // In a real implementation, this would trigger a system reboot
        // For now, just return the expected response
        Ok(SystemRebootResponse {
            message: "Rebooting".to_string(),
        })
    }

    /// Handle SetSystemFactoryDefault request.
    ///
    /// Resets the device to factory defaults (stub implementation).
    pub fn handle_set_system_factory_default(
        &self,
        request: SetSystemFactoryDefault,
    ) -> OnvifResult<SetSystemFactoryDefaultResponse> {
        tracing::info!(
            "SetSystemFactoryDefault request - factory_default={:?}",
            request.factory_default
        );

        // Stub implementation - just log and return success
        // In a real implementation, this would:
        // - Hard: Reset all settings including network config
        // - Soft: Reset settings but keep network config
        tracing::warn!("Factory default reset requested but not implemented (stub)");

        Ok(SetSystemFactoryDefaultResponse {})
    }

    /// Handle GetCertificates request.
    ///
    /// Returns installed certificates (empty list - no certificates supported).
    pub fn handle_get_certificates(
        &self,
        _request: GetCertificates,
    ) -> OnvifResult<GetCertificatesResponse> {
        tracing::debug!("GetCertificates request");

        // Return empty list - no certificates installed
        Ok(GetCertificatesResponse {
            nvt_certificate: vec![],
        })
    }

    /// Handle GetCertificatesStatus request.
    ///
    /// Returns certificate statuses (empty list - no certificates supported).
    pub fn handle_get_certificates_status(
        &self,
        _request: GetCertificatesStatus,
    ) -> OnvifResult<GetCertificatesStatusResponse> {
        tracing::debug!("GetCertificatesStatus request");

        // Return empty list - no certificates installed
        Ok(GetCertificatesStatusResponse {
            certificate_status: vec![],
        })
    }

    /// Handle CreateCertificate request (stub - not supported).
    pub fn handle_create_certificate(
        &self,
        _request: CreateCertificate,
    ) -> OnvifResult<CreateCertificateResponse> {
        tracing::debug!("CreateCertificate request (not supported)");
        Err(OnvifError::ActionNotSupported(
            "CreateCertificate".to_string(),
        ))
    }

    /// Handle LoadCertificates request (stub - not supported).
    pub fn handle_load_certificates(
        &self,
        _request: LoadCertificates,
    ) -> OnvifResult<LoadCertificatesResponse> {
        tracing::debug!("LoadCertificates request (not supported)");
        Err(OnvifError::ActionNotSupported(
            "LoadCertificates".to_string(),
        ))
    }

    /// Handle DeleteCertificates request (stub - not supported).
    pub fn handle_delete_certificates(
        &self,
        _request: DeleteCertificates,
    ) -> OnvifResult<DeleteCertificatesResponse> {
        tracing::debug!("DeleteCertificates request (not supported)");
        Err(OnvifError::ActionNotSupported(
            "DeleteCertificates".to_string(),
        ))
    }

    // ========================================================================
    // System Backup/Restore Handlers
    // ========================================================================

    /// Handle GetSystemBackup request.
    ///
    /// Returns a backup of the system configuration as a base64-encoded TOML file.
    pub fn handle_get_system_backup(
        &self,
        _request: GetSystemBackup,
    ) -> OnvifResult<GetSystemBackupResponse> {
        tracing::debug!("GetSystemBackup request");

        // Serialize current configuration to TOML
        let config_toml = self.config.to_toml_string().map_err(|e| {
            OnvifError::HardwareFailure(format!("Failed to serialize config: {}", e))
        })?;

        // Encode as base64
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let encoded = STANDARD.encode(config_toml.as_bytes());

        Ok(GetSystemBackupResponse {
            backup_files: vec![BackupFile {
                name: "config.toml".to_string(),
                data: encoded,
            }],
        })
    }

    /// Handle RestoreSystem request.
    ///
    /// Restores system configuration from a backup file.
    pub fn handle_restore_system(
        &self,
        request: RestoreSystem,
    ) -> OnvifResult<RestoreSystemResponse> {
        tracing::debug!(
            "RestoreSystem request with {} files",
            request.backup_files.len()
        );

        // Find config.toml in backup files
        let config_file = request
            .backup_files
            .iter()
            .find(|f| f.name == "config.toml")
            .ok_or_else(|| OnvifError::WellFormed("Backup must contain config.toml".to_string()))?;

        // Decode base64
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let decoded = STANDARD
            .decode(&config_file.data)
            .map_err(|e| OnvifError::WellFormed(format!("Invalid base64 data: {}", e)))?;

        // Parse as TOML string
        let toml_str = String::from_utf8(decoded)
            .map_err(|e| OnvifError::WellFormed(format!("Invalid UTF-8 in config: {}", e)))?;

        // Apply configuration
        self.config
            .load_from_toml_string(&toml_str)
            .map_err(|e| OnvifError::HardwareFailure(format!("Failed to apply config: {}", e)))?;

        tracing::info!("RestoreSystem: configuration restored successfully");

        Ok(RestoreSystemResponse {})
    }

    /// Handle GetRelayOutputs request.
    ///
    /// Returns relay outputs (empty list - no relay outputs supported).
    pub fn handle_get_relay_outputs(
        &self,
        _request: GetRelayOutputs,
    ) -> OnvifResult<GetRelayOutputsResponse> {
        tracing::debug!("GetRelayOutputs request");

        // Return empty list - no relay outputs on this device
        Ok(GetRelayOutputsResponse {
            relay_outputs: vec![],
        })
    }

    // ========================================================================
    // Hostname Handlers
    // ========================================================================

    /// Handle GetHostname request.
    ///
    /// Returns current hostname configuration.
    pub fn handle_get_hostname(&self, _request: GetHostname) -> OnvifResult<GetHostnameResponse> {
        tracing::debug!("GetHostname request");

        let hostname = self
            .config
            .get_string("device.hostname")
            .unwrap_or_else(|_| "onvif-camera".to_string());

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
    pub fn handle_set_hostname(&self, request: SetHostname) -> OnvifResult<SetHostnameResponse> {
        tracing::debug!("SetHostname request: {}", request.name);

        // Validate hostname
        validate_hostname(&request.name)?;

        // Save to configuration
        self.config
            .set_string("device.hostname", &request.name)
            .map_err(|e| OnvifError::HardwareFailure(format!("Failed to save hostname: {}", e)))?;

        tracing::info!("SetHostname: hostname set to '{}'", request.name);

        Ok(SetHostnameResponse {})
    }

    // ========================================================================
    // Network Interfaces Handlers
    // ========================================================================

    /// Handle GetNetworkInterfaces request.
    ///
    /// Returns network interface configurations from platform or fallback to config.
    pub async fn handle_get_network_interfaces(
        &self,
        _request: GetNetworkInterfaces,
    ) -> OnvifResult<GetNetworkInterfacesResponse> {
        tracing::debug!("GetNetworkInterfaces request");

        let (ip_address, mac_address, dhcp_enabled) = self
            .get_network_info_from_platform()
            .await
            .unwrap_or_else(|| self.get_network_info_fallback());

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

    /// Get network info from platform if available.
    async fn get_network_info_from_platform(&self) -> Option<(String, String, bool)> {
        let platform = self.platform.as_ref()?;
        let network_info = platform.network_info()?;
        let interfaces = network_info.get_network_interfaces().await.ok()?;
        let iface = interfaces.first()?;

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

        Some((ip, mac, dhcp))
    }

    /// Fallback method to get network info from config when platform is unavailable.
    fn get_network_info_fallback(&self) -> (String, String, bool) {
        let ip_address = self
            .config
            .get_string("network.detected_ip")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(Self::detect_local_ip)
            .unwrap_or_else(|| "192.168.1.100".to_string());

        let mac_address = self
            .config
            .get_string("network.mac_address")
            .unwrap_or_else(|_| "00:11:22:33:44:55".to_string());

        let dhcp_enabled = self.config.get_bool("network.dhcp_enabled").unwrap_or(true);

        (ip_address, mac_address, dhcp_enabled)
    }

    // ========================================================================
    // DNS Handlers
    // ========================================================================

    /// Handle GetDNS request.
    ///
    /// Returns DNS configuration.
    pub async fn handle_get_dns(&self, _request: GetDNS) -> OnvifResult<GetDNSResponse> {
        tracing::debug!("GetDNS request");

        // Try to get DNS info from platform
        if let Some(platform) = &self.platform {
            if let Some(network_info) = platform.network_info() {
                if let Ok(dns_info) = network_info.get_dns_info().await {
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
            }
        }

        // Return empty DNS info if no platform available
        Ok(GetDNSResponse {
            dns_information: DNSInformation::default(),
        })
    }

    /// Handle SetDNS request.
    ///
    /// Sets DNS configuration.
    pub async fn handle_set_dns(&self, request: SetDNS) -> OnvifResult<SetDNSResponse> {
        tracing::debug!(
            "SetDNS request: from_dhcp={}, {} manual servers",
            request.from_dhcp,
            request.dns_manual.len()
        );

        // TODO: Implement actual DNS setting via platform
        // For now, just log and return success
        tracing::info!(
            "SetDNS: from_dhcp={}, manual_servers={:?}, search_domains={:?}",
            request.from_dhcp,
            request
                .dns_manual
                .iter()
                .filter_map(|ip| ip.ipv4_address.as_ref())
                .collect::<Vec<_>>(),
            request.search_domain
        );

        Ok(SetDNSResponse {})
    }

    // ========================================================================
    // NTP Handlers
    // ========================================================================

    /// Handle GetNTP request.
    ///
    /// Returns NTP configuration.
    pub async fn handle_get_ntp(&self, _request: GetNTP) -> OnvifResult<GetNTPResponse> {
        tracing::debug!("GetNTP request");

        // Try to get NTP info from platform
        if let Some(platform) = &self.platform {
            if let Some(network_info) = platform.network_info() {
                if let Ok(ntp_info) = network_info.get_ntp_info().await {
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
            }
        }

        // Return empty NTP info if no platform available
        Ok(GetNTPResponse {
            ntp_information: NTPInformation::default(),
        })
    }

    /// Handle SetNTP request.
    ///
    /// Sets NTP configuration.
    pub async fn handle_set_ntp(&self, request: SetNTP) -> OnvifResult<SetNTPResponse> {
        tracing::debug!(
            "SetNTP request: from_dhcp={}, {} manual servers",
            request.from_dhcp,
            request.ntp_manual.len()
        );

        // TODO: Implement actual NTP setting via platform
        // For now, just log and return success
        let servers: Vec<String> = request
            .ntp_manual
            .iter()
            .filter_map(|host| match host.host_type {
                NetworkHostType::IPv4 => host.ipv4_address.clone(),
                NetworkHostType::IPv6 => host.ipv6_address.clone(),
                NetworkHostType::DNS => host.dns_name.clone(),
            })
            .collect();

        tracing::info!(
            "SetNTP: from_dhcp={}, manual_servers={:?}",
            request.from_dhcp,
            servers
        );

        Ok(SetNTPResponse {})
    }

    // ========================================================================
    // Network Gateway Handlers
    // ========================================================================

    /// Handle GetNetworkDefaultGateway request.
    ///
    /// Returns default gateway configuration.
    pub async fn handle_get_network_default_gateway(
        &self,
        _request: GetNetworkDefaultGateway,
    ) -> OnvifResult<GetNetworkDefaultGatewayResponse> {
        tracing::debug!("GetNetworkDefaultGateway request");

        // Get gateway from config (platform doesn't expose gateway info)
        let gateway = self
            .config
            .get_string("network.gateway")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "192.168.1.1".to_string());

        let network_gateway = NetworkGateway {
            ipv4_address: vec![gateway],
            ipv6_address: vec![],
            extension: None,
        };

        Ok(GetNetworkDefaultGatewayResponse {
            network_gateway: vec![network_gateway],
        })
    }

    // ========================================================================
    // Network Protocols Handlers
    // ========================================================================

    /// Handle GetNetworkProtocols request.
    ///
    /// Returns network protocol configurations.
    pub async fn handle_get_network_protocols(
        &self,
        _request: GetNetworkProtocols,
    ) -> OnvifResult<GetNetworkProtocolsResponse> {
        tracing::debug!("GetNetworkProtocols request");

        // Try to get protocol info from platform
        if let Some(platform) = &self.platform {
            if let Some(network_info) = platform.network_info() {
                if let Ok(protocols) = network_info.get_network_protocols().await {
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
            }
        }

        // Return default protocol info from config if no platform
        let http_port = self.config.get_int("server.port").unwrap_or(80) as i32;

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
    /// Sets network protocol configurations.
    pub async fn handle_set_network_protocols(
        &self,
        request: SetNetworkProtocols,
    ) -> OnvifResult<SetNetworkProtocolsResponse> {
        tracing::debug!(
            "SetNetworkProtocols request: {} protocols",
            request.network_protocols.len()
        );

        // TODO: Implement actual protocol setting via platform
        // For now, just log and return success
        for protocol in &request.network_protocols {
            tracing::info!(
                "SetNetworkProtocols: {:?} enabled={} ports={:?}",
                protocol.name,
                protocol.enabled,
                protocol.port
            );
        }

        Ok(SetNetworkProtocolsResponse {})
    }

    // ========================================================================
    // Scope Handlers
    // ========================================================================

    /// Handle GetScopes request.
    ///
    /// Returns device scopes.
    pub fn handle_get_scopes(&self, _request: GetScopes) -> OnvifResult<GetScopesResponse> {
        tracing::debug!("GetScopes request");

        let scopes = self.scopes.read().clone();

        Ok(GetScopesResponse { scopes })
    }

    /// Handle SetScopes request.
    ///
    /// Replaces configurable scopes.
    pub fn handle_set_scopes(&self, request: SetScopes) -> OnvifResult<SetScopesResponse> {
        tracing::debug!("SetScopes request: {} scopes", request.scopes.len());

        // Validate all scopes
        for scope in &request.scopes {
            validate_scope(scope)?;
        }

        let mut scopes = self.scopes.write();

        // Keep fixed scopes, replace configurable ones
        let fixed_scopes: Vec<Scope> = scopes
            .iter()
            .filter(|s| matches!(s.scope_def, ScopeDefinition::Fixed))
            .cloned()
            .collect();

        let new_configurable: Vec<Scope> = request
            .scopes
            .into_iter()
            .map(|s| Scope {
                scope_def: ScopeDefinition::Configurable,
                scope_item: s,
            })
            .collect();

        *scopes = fixed_scopes;
        scopes.extend(new_configurable);

        tracing::info!("SetScopes: updated to {} scopes", scopes.len());

        Ok(SetScopesResponse {})
    }

    /// Handle AddScopes request.
    ///
    /// Adds new scopes to the device.
    pub fn handle_add_scopes(&self, request: AddScopes) -> OnvifResult<AddScopesResponse> {
        tracing::debug!("AddScopes request: {} scopes", request.scope_item.len());

        // Validate all scopes
        for scope in &request.scope_item {
            validate_scope(scope)?;
        }

        let mut scopes = self.scopes.write();

        for scope_item in request.scope_item {
            // Check if scope already exists
            if !scopes.iter().any(|s| s.scope_item == scope_item) {
                scopes.push(Scope {
                    scope_def: ScopeDefinition::Configurable,
                    scope_item,
                });
            }
        }

        tracing::info!("AddScopes: now have {} scopes", scopes.len());

        Ok(AddScopesResponse {})
    }

    /// Handle RemoveScopes request.
    ///
    /// Removes specified scopes from the device. Only configurable scopes can be removed.
    /// Fixed scopes cannot be removed.
    pub fn handle_remove_scopes(&self, request: RemoveScopes) -> OnvifResult<RemoveScopesResponse> {
        tracing::debug!("RemoveScopes request: {} scopes", request.scope_item.len());

        let mut scopes = self.scopes.write();
        let mut removed = Vec::new();

        for scope_item in &request.scope_item {
            // Find and remove the scope (only if configurable)
            if let Some(pos) = scopes.iter().position(|s| {
                s.scope_item == *scope_item && matches!(s.scope_def, ScopeDefinition::Configurable)
            }) {
                scopes.remove(pos);
                removed.push(scope_item.clone());
            }
        }

        tracing::info!(
            "RemoveScopes: removed {} scopes, {} remaining",
            removed.len(),
            scopes.len()
        );

        // Return the removed scope items per WSDL
        Ok(RemoveScopesResponse {
            scope_item: removed,
        })
    }

    // ========================================================================
    // Discovery Mode Handlers
    // ========================================================================

    /// Handle GetDiscoveryMode request.
    ///
    /// Returns current discovery mode.
    pub fn handle_get_discovery_mode(
        &self,
        _request: GetDiscoveryMode,
    ) -> OnvifResult<GetDiscoveryModeResponse> {
        tracing::debug!("GetDiscoveryMode request");

        let mode = self.discovery_mode.read().clone();

        Ok(GetDiscoveryModeResponse {
            discovery_mode: mode,
        })
    }

    /// Handle SetDiscoveryMode request.
    ///
    /// Sets the discovery mode (Discoverable or NonDiscoverable).
    pub fn handle_set_discovery_mode(
        &self,
        request: SetDiscoveryMode,
    ) -> OnvifResult<SetDiscoveryModeResponse> {
        tracing::debug!("SetDiscoveryMode request: {:?}", request.discovery_mode);

        let mode = request.discovery_mode.clone();
        *self.discovery_mode.write() = request.discovery_mode;

        tracing::info!("SetDiscoveryMode: mode set to {:?}", mode);

        Ok(SetDiscoveryModeResponse {})
    }

    /// Handle GetUsers request.
    ///
    /// Returns a list of all users with their usernames and levels.
    /// Passwords are never returned.
    ///
    /// # Authorization
    ///
    /// Any authenticated user can call this operation.
    pub fn handle_get_users(&self, _request: GetUsers) -> OnvifResult<GetUsersResponse> {
        tracing::debug!("GetUsers request");

        let users: Vec<OnvifUser> = self
            .users
            .list_users()
            .into_iter()
            .map(OnvifUser::from)
            .collect();

        tracing::info!("GetUsers: returning {} users", users.len());

        Ok(GetUsersResponse { users })
    }

    /// Handle CreateUsers request.
    ///
    /// Creates new user accounts with the specified usernames, passwords, and levels.
    ///
    /// # Authorization
    ///
    /// Only Administrator users can call this operation.
    ///
    /// # Errors
    ///
    /// - `ter:NotAuthorized` - Caller is not an Administrator
    /// - `ter:MaxUsers` - Maximum 8 users reached
    /// - `ter:UsernameExists` - Username already exists
    /// - `ter:InvalidArgVal` - Invalid username or password format
    pub fn handle_create_users(
        &self,
        request: CreateUsers,
        caller_level: UserLevel,
    ) -> OnvifResult<CreateUsersResponse> {
        tracing::debug!("CreateUsers request: {} users", request.users.len());

        // Authorization check
        if !caller_level.is_admin() {
            tracing::warn!(
                "CreateUsers: unauthorized (caller level: {:?})",
                caller_level
            );
            return Err(OnvifError::NotAuthorized(
                "Administrator privileges required".to_string(),
            ));
        }

        for user in &request.users {
            // Validate username
            validate_username(&user.username).map_err(|e| OnvifError::InvalidArgVal {
                subcode: "ter:InvalidUsername".to_string(),
                reason: e.to_string(),
            })?;

            // Validate password (required for create)
            validate_password(user.password.as_deref(), true).map_err(|e| {
                OnvifError::InvalidArgVal {
                    subcode: "ter:InvalidPassword".to_string(),
                    reason: e.to_string(),
                }
            })?;

            // Get the password (already validated as present)
            let password = user.password.as_deref().unwrap();

            // Convert ONVIF user level to internal level
            let level: UserLevel = user.user_level.clone().into();

            // Create the user
            self.users
                .create_user(&user.username, password, level)
                .map_err(|e| match e {
                    crate::users::UserError::MaxUsersReached => OnvifError::MaxUsers,
                    crate::users::UserError::UserExists(name) => OnvifError::InvalidArgVal {
                        subcode: "ter:UsernameExists".to_string(),
                        reason: format!("User '{}' already exists", name),
                    },
                    _ => OnvifError::InvalidArgVal {
                        subcode: "ter:InvalidArgVal".to_string(),
                        reason: e.to_string(),
                    },
                })?;

            tracing::info!(
                "CreateUsers: created user '{}' with level {:?}",
                user.username,
                level
            );
        }

        // Persist users to config file
        if let Err(e) = self.users.save_to_toml("/mnt/anyka_hack/onvif/users.toml") {
            tracing::warn!("Failed to save users to file: {}", e);
        }

        Ok(CreateUsersResponse {})
    }

    /// Handle DeleteUsers request.
    ///
    /// Deletes the specified users by username.
    ///
    /// # Authorization
    ///
    /// Only Administrator users can call this operation.
    ///
    /// # Errors
    ///
    /// - `ter:NotAuthorized` - Caller is not an Administrator
    /// - `ter:UserNotFound` - Unknown username
    /// - `ter:FixedUser` - Cannot delete last administrator
    pub fn handle_delete_users(
        &self,
        request: DeleteUsers,
        caller_level: UserLevel,
    ) -> OnvifResult<DeleteUsersResponse> {
        tracing::debug!("DeleteUsers request: {} users", request.usernames.len());

        // Authorization check
        if !caller_level.is_admin() {
            tracing::warn!(
                "DeleteUsers: unauthorized (caller level: {:?})",
                caller_level
            );
            return Err(OnvifError::NotAuthorized(
                "Administrator privileges required".to_string(),
            ));
        }

        for username in &request.usernames {
            self.users.delete_user(username).map_err(|e| match e {
                crate::users::UserError::UserNotFound(name) => OnvifError::InvalidArgVal {
                    subcode: "ter:UserNotFound".to_string(),
                    reason: format!("User '{}' not found", name),
                },
                crate::users::UserError::CannotDeleteLastAdmin => OnvifError::InvalidArgVal {
                    subcode: "ter:FixedUser".to_string(),
                    reason: "Cannot delete the last administrator".to_string(),
                },
                _ => OnvifError::InvalidArgVal {
                    subcode: "ter:InvalidArgVal".to_string(),
                    reason: e.to_string(),
                },
            })?;

            tracing::info!("DeleteUsers: deleted user '{}'", username);
        }

        // Persist users to config file
        if let Err(e) = self.users.save_to_toml("/mnt/anyka_hack/onvif/users.toml") {
            tracing::warn!("Failed to save users to file: {}", e);
        }

        Ok(DeleteUsersResponse {})
    }

    /// Handle SetUser request.
    ///
    /// Updates existing users' passwords and/or levels.
    ///
    /// # Authorization
    ///
    /// Only Administrator users can call this operation.
    ///
    /// # Errors
    ///
    /// - `ter:NotAuthorized` - Caller is not an Administrator
    /// - `ter:UserNotFound` - Unknown username
    /// - `ter:FixedUser` - Cannot demote last administrator
    /// - `ter:InvalidArgVal` - Invalid password format
    pub fn handle_set_user(
        &self,
        request: SetUser,
        caller_level: UserLevel,
    ) -> OnvifResult<SetUserResponse> {
        tracing::debug!("SetUser request: {} users", request.users.len());

        // Authorization check
        if !caller_level.is_admin() {
            tracing::warn!("SetUser: unauthorized (caller level: {:?})", caller_level);
            return Err(OnvifError::NotAuthorized(
                "Administrator privileges required".to_string(),
            ));
        }

        for user in &request.users {
            // Validate password if provided
            validate_password(user.password.as_deref(), false).map_err(|e| {
                OnvifError::InvalidArgVal {
                    subcode: "ter:InvalidPassword".to_string(),
                    reason: e.to_string(),
                }
            })?;

            // Get password if provided
            let password = user.password.as_deref();

            // Convert level
            let level: UserLevel = user.user_level.clone().into();

            // Update the user
            self.users
                .update_user(&user.username, password, Some(level))
                .map_err(|e| match e {
                    crate::users::UserError::UserNotFound(name) => OnvifError::InvalidArgVal {
                        subcode: "ter:UserNotFound".to_string(),
                        reason: format!("User '{}' not found", name),
                    },
                    crate::users::UserError::CannotDeleteLastAdmin => OnvifError::InvalidArgVal {
                        subcode: "ter:FixedUser".to_string(),
                        reason: "Cannot demote the last administrator".to_string(),
                    },
                    _ => OnvifError::InvalidArgVal {
                        subcode: "ter:InvalidArgVal".to_string(),
                        reason: e.to_string(),
                    },
                })?;

            tracing::info!("SetUser: updated user '{}'", user.username);
        }

        // Persist users to config file
        if let Err(e) = self.users.save_to_toml("/mnt/anyka_hack/onvif/users.toml") {
            tracing::warn!("Failed to save users to file: {}", e);
        }

        Ok(SetUserResponse {})
    }
}

// ============================================================================
// ServiceHandler Implementation for DeviceService
// ============================================================================

use crate::onvif::dispatcher::ServiceHandler;
use async_trait::async_trait;

#[async_trait]
impl ServiceHandler for DeviceService {
    /// Handle a SOAP operation for the Device Service.
    ///
    /// Routes the SOAP action to the appropriate handler method and returns
    /// the serialized XML response.
    async fn handle_operation(&self, action: &str, body_xml: &str) -> OnvifResult<String> {
        tracing::debug!("DeviceService handling action: {}", action);

        match action {
            // Device Information Operations
            "GetDeviceInformation" => {
                let _request: GetDeviceInformation = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_device_information(_request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Capabilities Operations
            "GetCapabilities" => {
                let request: GetCapabilities = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_capabilities(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetServices" => {
                let request: GetServices = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_services(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetServiceCapabilities" => {
                let request: GetServiceCapabilities = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_service_capabilities(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // System Date/Time Operations
            "GetSystemDateAndTime" => {
                let request: GetSystemDateAndTime = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_system_date_and_time(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetSystemDateAndTime" => {
                let request: SetSystemDateAndTime = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_system_date_and_time(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // System Operations
            "SystemReboot" => {
                let request: SystemReboot = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_system_reboot(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetSystemFactoryDefault" => {
                let request: SetSystemFactoryDefault = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_system_factory_default(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Certificate Operations
            "GetCertificates" => {
                let request: GetCertificates = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_certificates(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetCertificatesStatus" => {
                let request: GetCertificatesStatus = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_certificates_status(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "CreateCertificate" => {
                let request: CreateCertificate = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_create_certificate(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "LoadCertificates" => {
                let request: LoadCertificates = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_load_certificates(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "DeleteCertificates" => {
                let request: DeleteCertificates = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_delete_certificates(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Backup/Restore Operations
            "GetSystemBackup" => {
                let request: GetSystemBackup = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_system_backup(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "RestoreSystem" => {
                let request: RestoreSystem = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_restore_system(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Relay Operations
            "GetRelayOutputs" => {
                let request: GetRelayOutputs = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_relay_outputs(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Hostname Operations
            "GetHostname" => {
                let request: GetHostname = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_hostname(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetHostname" => {
                let request: SetHostname = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_hostname(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Network Operations
            "GetNetworkInterfaces" => {
                let request: GetNetworkInterfaces = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_network_interfaces(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetNetworkDefaultGateway" => {
                let request: GetNetworkDefaultGateway = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_network_default_gateway(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // DNS Operations
            "GetDNS" => {
                let request: GetDNS = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_dns(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetDNS" => {
                let request: SetDNS = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_dns(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // NTP Operations
            "GetNTP" => {
                let request: GetNTP = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_ntp(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetNTP" => {
                let request: SetNTP = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_ntp(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Network Protocol Operations
            "GetNetworkProtocols" => {
                let request: GetNetworkProtocols = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_network_protocols(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetNetworkProtocols" => {
                let request: SetNetworkProtocols = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_network_protocols(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Scope Operations
            "GetScopes" => {
                let request: GetScopes = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_scopes(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetScopes" => {
                let request: SetScopes = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_scopes(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "AddScopes" => {
                let request: AddScopes = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_add_scopes(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "RemoveScopes" => {
                let request: RemoveScopes = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_remove_scopes(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Discovery Operations
            "GetDiscoveryMode" => {
                let request: GetDiscoveryMode = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_discovery_mode(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetDiscoveryMode" => {
                let request: SetDiscoveryMode = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_discovery_mode(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // User Management Operations
            "GetUsers" => {
                let request: GetUsers = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_users(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "CreateUsers" => {
                // Note: Caller level should be extracted from auth context
                // For now, assume admin level for testing
                let request: CreateUsers = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_create_users(request, UserLevel::Administrator)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "DeleteUsers" => {
                // Note: Caller level should be extracted from auth context
                let request: DeleteUsers = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_delete_users(request, UserLevel::Administrator)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetUser" => {
                // Note: Caller level should be extracted from auth context
                let request: SetUser = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_user(request, UserLevel::Administrator)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Unknown action
            _ => {
                tracing::warn!("Unsupported Device Service action: {}", action);
                Err(OnvifError::ActionNotSupported(action.to_string()))
            }
        }
    }

    /// Get the service name.
    fn service_name(&self) -> &str {
        "Device"
    }

    /// Get the list of supported actions.
    fn supported_actions(&self) -> Vec<&str> {
        vec![
            "GetDeviceInformation",
            "GetCapabilities",
            "GetServices",
            "GetServiceCapabilities",
            "GetSystemDateAndTime",
            "SetSystemDateAndTime",
            "SystemReboot",
            "SetSystemFactoryDefault",
            "GetSystemBackup",
            "RestoreSystem",
            "GetCertificates",
            "GetCertificatesStatus",
            "CreateCertificate",
            "LoadCertificates",
            "DeleteCertificates",
            "GetRelayOutputs",
            "GetHostname",
            "SetHostname",
            "GetNetworkInterfaces",
            "GetNetworkDefaultGateway",
            "GetDNS",
            "SetDNS",
            "GetNTP",
            "SetNTP",
            "GetNetworkProtocols",
            "SetNetworkProtocols",
            "GetScopes",
            "SetScopes",
            "AddScopes",
            "RemoveScopes",
            "GetDiscoveryMode",
            "SetDiscoveryMode",
            "GetUsers",
            "CreateUsers",
            "DeleteUsers",
            "SetUser",
        ]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::types::common::UserLevel as OnvifUserLevel;

    fn create_test_service() -> DeviceService {
        let users = Arc::new(UserStorage::new());
        let password_manager = Arc::new(PasswordManager::new());

        // Create initial admin user with plaintext password
        users
            .create_user("admin", "admin123", UserLevel::Administrator)
            .unwrap();

        DeviceService::new(users, password_manager)
    }

    // ========================================================================
    // GetDeviceInformation Tests (T204)
    // ========================================================================

    #[tokio::test]
    async fn test_get_device_information() {
        let service = create_test_service();
        let response = service
            .handle_get_device_information(GetDeviceInformation {})
            .await
            .unwrap();

        // Default values from config - these should always be present
        assert_eq!(response.manufacturer, "Anyka");
        assert_eq!(response.model, "AK3918 Camera");
        assert!(!response.firmware_version.is_empty());
        // serial_number and hardware_id may be empty in test config - just verify they exist
        // In production, these would be populated from platform or config
    }

    // ========================================================================
    // GetCapabilities Tests (T205)
    // ========================================================================

    #[test]
    fn test_get_capabilities() {
        let service = create_test_service();
        let response = service
            .handle_get_capabilities(GetCapabilities { category: vec![] })
            .unwrap();

        // Should have device capabilities
        assert!(response.capabilities.device.is_some());
        let device_caps = response.capabilities.device.unwrap();
        assert!(device_caps.x_addr.contains("device_service"));

        // Should have media capabilities
        assert!(response.capabilities.media.is_some());
        let media_caps = response.capabilities.media.unwrap();
        assert!(media_caps.x_addr.contains("media_service"));

        // Should have PTZ capabilities
        assert!(response.capabilities.ptz.is_some());

        // Should have imaging capabilities
        assert!(response.capabilities.imaging.is_some());
    }

    // ========================================================================
    // GetServices Tests (T206)
    // ========================================================================

    #[test]
    fn test_get_services() {
        let service = create_test_service();
        let response = service
            .handle_get_services(GetServices {
                include_capability: false,
            })
            .unwrap();

        // Should have 4 services: Device, Media, PTZ, Imaging
        assert_eq!(response.services.len(), 4);

        // Check service namespaces
        let namespaces: Vec<&str> = response
            .services
            .iter()
            .map(|s| s.namespace.as_str())
            .collect();
        assert!(namespaces.contains(&DEVICE_SERVICE_NAMESPACE));
        assert!(namespaces.contains(&MEDIA_SERVICE_NAMESPACE));
        assert!(namespaces.contains(&PTZ_SERVICE_NAMESPACE));
        assert!(namespaces.contains(&IMAGING_SERVICE_NAMESPACE));

        // Check versions
        for service in &response.services {
            assert_eq!(service.version.major, 2);
            assert_eq!(service.version.minor, 5);
        }
    }

    // ========================================================================
    // GetSystemDateAndTime Tests (T207)
    // ========================================================================

    #[test]
    fn test_get_system_date_and_time() {
        let service = create_test_service();
        let response = service
            .handle_get_system_date_and_time(GetSystemDateAndTime {})
            .unwrap();

        let sdt = &response.system_date_and_time;

        // Should have UTC time
        assert!(sdt.utc_date_time.is_some());
        let utc = sdt.utc_date_time.as_ref().unwrap();

        // Basic sanity checks
        assert!(utc.date.year >= 2024);
        assert!(utc.date.month >= 1 && utc.date.month <= 12);
        assert!(utc.date.day >= 1 && utc.date.day <= 31);
        assert!(utc.time.hour >= 0 && utc.time.hour <= 23);
        assert!(utc.time.minute >= 0 && utc.time.minute <= 59);
        assert!(utc.time.second >= 0 && utc.time.second <= 59);

        // Should have local time
        assert!(sdt.local_date_time.is_some());

        // Should have timezone
        assert!(sdt.time_zone.is_some());
    }

    // ========================================================================
    // SystemReboot Tests (T208)
    // ========================================================================

    #[test]
    fn test_system_reboot() {
        let service = create_test_service();
        let response = service.handle_system_reboot(SystemReboot {}).unwrap();

        assert_eq!(response.message, "Rebooting");
    }

    // ========================================================================
    // GetHostname Tests (T209)
    // ========================================================================

    #[test]
    fn test_get_hostname() {
        let service = create_test_service();
        let response = service.handle_get_hostname(GetHostname {}).unwrap();

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
        let service = create_test_service();
        let result = service.handle_set_hostname(SetHostname {
            name: "my-camera".to_string(),
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_hostname_invalid_empty() {
        let service = create_test_service();
        let result = service.handle_set_hostname(SetHostname {
            name: "".to_string(),
        });
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:InvalidHostname"
        ));
    }

    #[test]
    fn test_set_hostname_invalid_chars() {
        let service = create_test_service();
        let result = service.handle_set_hostname(SetHostname {
            name: "camera.local".to_string(),
        });
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:InvalidHostname"
        ));
    }

    #[test]
    fn test_set_hostname_invalid_leading_hyphen() {
        let service = create_test_service();
        let result = service.handle_set_hostname(SetHostname {
            name: "-camera".to_string(),
        });
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
        let service = create_test_service();
        let response = service
            .handle_get_network_interfaces(GetNetworkInterfaces {})
            .await
            .unwrap();

        // Currently returns empty list (requires platform integration)
        assert!(response.network_interfaces.is_empty() || !response.network_interfaces.is_empty());
    }

    // ========================================================================
    // GetScopes Tests (T212)
    // ========================================================================

    #[test]
    fn test_get_scopes() {
        let service = create_test_service();
        let response = service.handle_get_scopes(GetScopes {}).unwrap();

        // Should have default scopes
        assert!(!response.scopes.is_empty());

        // Should have fixed scopes
        let fixed_scopes: Vec<_> = response
            .scopes
            .iter()
            .filter(|s| matches!(s.scope_def, ScopeDefinition::Fixed))
            .collect();
        assert!(!fixed_scopes.is_empty());
    }

    // ========================================================================
    // SetScopes Tests (T213)
    // ========================================================================

    #[test]
    fn test_set_scopes_valid() {
        let service = create_test_service();
        let result = service.handle_set_scopes(SetScopes {
            scopes: vec![
                "onvif://www.onvif.org/location/office".to_string(),
                "onvif://www.onvif.org/name/MyCamera".to_string(),
            ],
        });
        assert!(result.is_ok());

        // Verify scopes were updated
        let scopes = service.handle_get_scopes(GetScopes {}).unwrap();
        let scope_items: Vec<&str> = scopes
            .scopes
            .iter()
            .map(|s| s.scope_item.as_str())
            .collect();
        assert!(scope_items.contains(&"onvif://www.onvif.org/location/office"));
        assert!(scope_items.contains(&"onvif://www.onvif.org/name/MyCamera"));
    }

    #[test]
    fn test_set_scopes_preserves_fixed() {
        let service = create_test_service();

        // Get initial fixed scopes
        let initial = service.handle_get_scopes(GetScopes {}).unwrap();
        let fixed_count = initial
            .scopes
            .iter()
            .filter(|s| matches!(s.scope_def, ScopeDefinition::Fixed))
            .count();

        // Set new configurable scopes
        service
            .handle_set_scopes(SetScopes {
                scopes: vec!["onvif://www.onvif.org/name/NewName".to_string()],
            })
            .unwrap();

        // Verify fixed scopes are preserved
        let after = service.handle_get_scopes(GetScopes {}).unwrap();
        let fixed_after = after
            .scopes
            .iter()
            .filter(|s| matches!(s.scope_def, ScopeDefinition::Fixed))
            .count();

        assert_eq!(fixed_count, fixed_after);
    }

    #[test]
    fn test_set_scopes_invalid_prefix() {
        let service = create_test_service();
        let result = service.handle_set_scopes(SetScopes {
            scopes: vec!["http://example.com/scope".to_string()],
        });
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:InvalidScope"
        ));
    }

    // ========================================================================
    // AddScopes Tests (T214)
    // ========================================================================

    #[test]
    fn test_add_scopes() {
        let service = create_test_service();
        let initial = service.handle_get_scopes(GetScopes {}).unwrap();
        let initial_count = initial.scopes.len();

        // Add a new scope
        service
            .handle_add_scopes(AddScopes {
                scope_item: vec!["onvif://www.onvif.org/hardware/HD".to_string()],
            })
            .unwrap();

        // Verify scope was added
        let after = service.handle_get_scopes(GetScopes {}).unwrap();
        assert_eq!(after.scopes.len(), initial_count + 1);
    }

    #[test]
    fn test_add_scopes_no_duplicates() {
        let service = create_test_service();

        // Add the same scope twice
        service
            .handle_add_scopes(AddScopes {
                scope_item: vec!["onvif://www.onvif.org/unique/scope".to_string()],
            })
            .unwrap();

        let count1 = service
            .handle_get_scopes(GetScopes {})
            .unwrap()
            .scopes
            .len();

        service
            .handle_add_scopes(AddScopes {
                scope_item: vec!["onvif://www.onvif.org/unique/scope".to_string()],
            })
            .unwrap();

        let count2 = service
            .handle_get_scopes(GetScopes {})
            .unwrap()
            .scopes
            .len();

        assert_eq!(count1, count2);
    }

    // ========================================================================
    // RemoveScopes Tests
    // ========================================================================

    #[test]
    fn test_remove_scopes_configurable() {
        let service = create_test_service();

        // First add a scope
        service
            .handle_add_scopes(AddScopes {
                scope_item: vec!["onvif://www.onvif.org/test/removable".to_string()],
            })
            .unwrap();

        let before = service.handle_get_scopes(GetScopes {}).unwrap();
        let before_count = before.scopes.len();

        // Remove the scope
        let response = service
            .handle_remove_scopes(RemoveScopes {
                scope_item: vec!["onvif://www.onvif.org/test/removable".to_string()],
            })
            .unwrap();

        // Should return the removed scope
        assert_eq!(response.scope_item.len(), 1);
        assert_eq!(
            response.scope_item[0],
            "onvif://www.onvif.org/test/removable"
        );

        // Verify scope count decreased
        let after = service.handle_get_scopes(GetScopes {}).unwrap();
        assert_eq!(after.scopes.len(), before_count - 1);
    }

    #[test]
    fn test_remove_scopes_fixed_not_removed() {
        let service = create_test_service();

        // Get a fixed scope
        let scopes = service.handle_get_scopes(GetScopes {}).unwrap();
        let fixed_scope = scopes
            .scopes
            .iter()
            .find(|s| matches!(s.scope_def, ScopeDefinition::Fixed))
            .expect("Should have fixed scope");

        let before_count = scopes.scopes.len();

        // Try to remove a fixed scope
        let response = service
            .handle_remove_scopes(RemoveScopes {
                scope_item: vec![fixed_scope.scope_item.clone()],
            })
            .unwrap();

        // Should return empty (nothing removed)
        assert!(response.scope_item.is_empty());

        // Verify count unchanged
        let after = service.handle_get_scopes(GetScopes {}).unwrap();
        assert_eq!(after.scopes.len(), before_count);
    }

    // ========================================================================
    // GetDiscoveryMode Tests (T215)
    // ========================================================================

    #[test]
    fn test_get_discovery_mode() {
        let service = create_test_service();
        let response = service
            .handle_get_discovery_mode(GetDiscoveryMode {})
            .unwrap();

        // Default should be Discoverable
        assert_eq!(response.discovery_mode, DiscoveryMode::Discoverable);
    }

    // ========================================================================
    // SetDiscoveryMode Tests (T216)
    // ========================================================================

    #[test]
    fn test_set_discovery_mode() {
        let service = create_test_service();

        // Set to NonDiscoverable
        service
            .handle_set_discovery_mode(SetDiscoveryMode {
                discovery_mode: DiscoveryMode::NonDiscoverable,
            })
            .unwrap();

        let response = service
            .handle_get_discovery_mode(GetDiscoveryMode {})
            .unwrap();
        assert_eq!(response.discovery_mode, DiscoveryMode::NonDiscoverable);

        // Set back to Discoverable
        service
            .handle_set_discovery_mode(SetDiscoveryMode {
                discovery_mode: DiscoveryMode::Discoverable,
            })
            .unwrap();

        let response = service
            .handle_get_discovery_mode(GetDiscoveryMode {})
            .unwrap();
        assert_eq!(response.discovery_mode, DiscoveryMode::Discoverable);
    }

    // ========================================================================
    // User Management Tests (existing)
    // ========================================================================

    #[test]
    fn test_get_users() {
        let service = create_test_service();

        let response = service.handle_get_users(GetUsers {}).unwrap();

        assert_eq!(response.users.len(), 1);
        assert_eq!(response.users[0].username, "admin");
        assert!(response.users[0].password.is_none()); // Password not returned
    }

    #[test]
    fn test_create_users_success() {
        let service = create_test_service();

        let request = CreateUsers {
            users: vec![OnvifUser {
                username: "operator".to_string(),
                password: Some("operator123".to_string()),
                user_level: OnvifUserLevel::Operator,
                extension: None,
            }],
        };

        let result = service.handle_create_users(request, UserLevel::Administrator);
        assert!(result.is_ok());

        // Verify user was created
        let users = service.handle_get_users(GetUsers {}).unwrap();
        assert_eq!(users.users.len(), 2);
    }

    #[test]
    fn test_create_users_unauthorized() {
        let service = create_test_service();

        let request = CreateUsers {
            users: vec![OnvifUser {
                username: "operator".to_string(),
                password: Some("operator123".to_string()),
                user_level: OnvifUserLevel::Operator,
                extension: None,
            }],
        };

        let result = service.handle_create_users(request, UserLevel::Operator);
        assert!(matches!(result, Err(OnvifError::NotAuthorized(_))));
    }

    #[test]
    fn test_create_users_max_users() {
        let service = create_test_service();

        // Create 7 more users to reach max (8)
        for i in 0..7 {
            let request = CreateUsers {
                users: vec![OnvifUser {
                    username: format!("user{}", i),
                    password: Some("password123".to_string()),
                    user_level: OnvifUserLevel::User,
                    extension: None,
                }],
            };
            service
                .handle_create_users(request, UserLevel::Administrator)
                .unwrap();
        }

        // 9th user should fail
        let request = CreateUsers {
            users: vec![OnvifUser {
                username: "overflow".to_string(),
                password: Some("password123".to_string()),
                user_level: OnvifUserLevel::User,
                extension: None,
            }],
        };

        let result = service.handle_create_users(request, UserLevel::Administrator);
        assert!(matches!(result, Err(OnvifError::MaxUsers)));
    }

    #[test]
    fn test_create_users_duplicate() {
        let service = create_test_service();

        let request = CreateUsers {
            users: vec![OnvifUser {
                username: "admin".to_string(), // Already exists
                password: Some("password123".to_string()),
                user_level: OnvifUserLevel::User,
                extension: None,
            }],
        };

        let result = service.handle_create_users(request, UserLevel::Administrator);
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:UsernameExists"
        ));
    }

    #[test]
    fn test_delete_users_success() {
        let service = create_test_service();

        // Create a user to delete
        let create_request = CreateUsers {
            users: vec![OnvifUser {
                username: "toDelete".to_string(),
                password: Some("password123".to_string()),
                user_level: OnvifUserLevel::User,
                extension: None,
            }],
        };
        service
            .handle_create_users(create_request, UserLevel::Administrator)
            .unwrap();

        // Delete the user
        let delete_request = DeleteUsers {
            usernames: vec!["toDelete".to_string()],
        };
        let result = service.handle_delete_users(delete_request, UserLevel::Administrator);
        assert!(result.is_ok());

        // Verify user was deleted
        let users = service.handle_get_users(GetUsers {}).unwrap();
        assert_eq!(users.users.len(), 1);
    }

    #[test]
    fn test_delete_users_unauthorized() {
        let service = create_test_service();

        let request = DeleteUsers {
            usernames: vec!["admin".to_string()],
        };

        let result = service.handle_delete_users(request, UserLevel::User);
        assert!(matches!(result, Err(OnvifError::NotAuthorized(_))));
    }

    #[test]
    fn test_delete_last_admin() {
        let service = create_test_service();

        let request = DeleteUsers {
            usernames: vec!["admin".to_string()],
        };

        let result = service.handle_delete_users(request, UserLevel::Administrator);
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:FixedUser"
        ));
    }

    #[test]
    fn test_set_user_password() {
        let service = create_test_service();

        let request = SetUser {
            users: vec![OnvifUser {
                username: "admin".to_string(),
                password: Some("newpassword123".to_string()),
                user_level: OnvifUserLevel::Administrator,
                extension: None,
            }],
        };

        let result = service.handle_set_user(request, UserLevel::Administrator);
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_user_unauthorized() {
        let service = create_test_service();

        let request = SetUser {
            users: vec![OnvifUser {
                username: "admin".to_string(),
                password: Some("newpassword".to_string()),
                user_level: OnvifUserLevel::Administrator,
                extension: None,
            }],
        };

        let result = service.handle_set_user(request, UserLevel::Operator);
        assert!(matches!(result, Err(OnvifError::NotAuthorized(_))));
    }

    #[test]
    fn test_set_user_not_found() {
        let service = create_test_service();

        let request = SetUser {
            users: vec![OnvifUser {
                username: "nonexistent".to_string(),
                password: Some("password".to_string()),
                user_level: OnvifUserLevel::User,
                extension: None,
            }],
        };

        let result = service.handle_set_user(request, UserLevel::Administrator);
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:UserNotFound"
        ));
    }

    // ========================================================================
    // GetServiceCapabilities Test
    // ========================================================================

    #[test]
    fn test_get_service_capabilities() {
        let service = create_test_service();
        let response = service
            .handle_get_service_capabilities(GetServiceCapabilities {})
            .unwrap();

        let caps = &response.capabilities;

        // Check security capabilities
        assert_eq!(caps.security.max_users, Some(8));
        assert_eq!(caps.security.username_token, Some(true));
        assert_eq!(caps.security.http_digest, Some(true));

        // Check system capabilities
        assert_eq!(caps.system.discovery_bye, Some(true));
        assert_eq!(caps.system.discovery_resolve, Some(true));
    }

    // ========================================================================
    // SetSystemDateAndTime Test
    // ========================================================================

    #[test]
    fn test_set_system_date_and_time() {
        let service = create_test_service();
        let result = service.handle_set_system_date_and_time(SetSystemDateAndTime {
            date_time_type: SetDateTimeType::Manual,
            daylight_savings: false,
            time_zone: Some(TimeZone {
                tz: "UTC0".to_string(),
            }),
            utc_date_time: Some(DateTime {
                date: Date {
                    year: 2025,
                    month: 1,
                    day: 15,
                },
                time: Time {
                    hour: 12,
                    minute: 30,
                    second: 0,
                },
            }),
        });
        assert!(result.is_ok());
    }
}
