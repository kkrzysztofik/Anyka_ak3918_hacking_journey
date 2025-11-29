//! Device Service types from devicemgmt.wsdl (tds:* namespace).
//!
//! This module contains request/response types for the ONVIF Device Service.
//! These types are defined in the `http://www.onvif.org/ver10/device/wsdl` namespace.
//!
//! # Operations
//!
//! ## Device Information
//! - `GetDeviceInformation` - Get device manufacturer, model, firmware version
//! - `GetServices` - Get list of available services
//! - `GetCapabilities` - Get device capabilities
//! - `GetServiceCapabilities` - Get device service capabilities
//!
//! ## System
//! - `GetSystemDateAndTime` / `SetSystemDateAndTime` - Date/time management
//! - `SystemReboot` - Reboot the device
//! - `SetSystemFactoryDefault` - Reset to factory defaults
//!
//! ## Network
//! - `GetHostname` / `SetHostname` - Hostname management
//! - `GetNetworkInterfaces` - Network interface information
//! - `GetDNS` / `SetDNS` - DNS configuration
//!
//! ## Discovery
//! - `GetScopes` / `SetScopes` / `AddScopes` / `RemoveScopes` - Scope management
//! - `GetDiscoveryMode` / `SetDiscoveryMode` - Discovery mode configuration
//!
//! ## Users
//! - `GetUsers` / `CreateUsers` / `DeleteUsers` / `SetUser` - User management

use serde::{Deserialize, Serialize};

use super::Extension;
use super::common::{
    DateTime, DiscoveryMode, FactoryDefaultType, OnvifVersion, ReferenceToken, Scope,
    SetDateTimeType, SystemDateTime, TimeZone, User,
};

// ============================================================================
// GetDeviceInformation
// ============================================================================

/// GetDeviceInformation request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetDeviceInformation")]
pub struct GetDeviceInformation {}

/// GetDeviceInformation response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetDeviceInformationResponse")]
pub struct GetDeviceInformationResponse {
    /// Device manufacturer.
    #[serde(rename = "Manufacturer")]
    pub manufacturer: String,

    /// Device model.
    #[serde(rename = "Model")]
    pub model: String,

    /// Firmware version.
    #[serde(rename = "FirmwareVersion")]
    pub firmware_version: String,

    /// Serial number.
    #[serde(rename = "SerialNumber")]
    pub serial_number: String,

    /// Hardware ID.
    #[serde(rename = "HardwareId")]
    pub hardware_id: String,
}

impl Default for GetDeviceInformationResponse {
    fn default() -> Self {
        Self {
            manufacturer: "ONVIF".to_string(),
            model: "Unknown".to_string(),
            firmware_version: "1.0.0".to_string(),
            serial_number: "000000000000".to_string(),
            hardware_id: "00001".to_string(),
        }
    }
}

// ============================================================================
// GetServices
// ============================================================================

/// GetServices request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetServices")]
pub struct GetServices {
    /// Include service capabilities in response.
    #[serde(rename = "IncludeCapability")]
    pub include_capability: bool,
}

impl Default for GetServices {
    fn default() -> Self {
        Self {
            include_capability: false,
        }
    }
}

/// GetServices response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetServicesResponse")]
pub struct GetServicesResponse {
    /// List of available services.
    #[serde(rename = "Service", default)]
    pub services: Vec<Service>,
}

/// Service information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Service {
    /// Service namespace URI.
    #[serde(rename = "Namespace")]
    pub namespace: String,

    /// Service endpoint address.
    #[serde(rename = "XAddr")]
    pub x_addr: String,

    /// Service capabilities (optional, included if requested).
    #[serde(
        rename = "Capabilities",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub capabilities: Option<ServiceCapabilities>,

    /// Service version.
    #[serde(rename = "Version")]
    pub version: OnvifVersion,
}

/// Service capabilities wrapper (contains any capabilities).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ServiceCapabilities {
    /// Raw capabilities XML content.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "$value")]
    pub content: Option<String>,
}

// ============================================================================
// GetServiceCapabilities
// ============================================================================

/// GetServiceCapabilities request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetServiceCapabilities")]
pub struct GetServiceCapabilities {}

/// GetServiceCapabilities response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetServiceCapabilitiesResponse")]
pub struct GetServiceCapabilitiesResponse {
    /// Device service capabilities.
    #[serde(rename = "Capabilities")]
    pub capabilities: DeviceServiceCapabilities,
}

/// Device service capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceServiceCapabilities {
    /// Network capabilities.
    #[serde(rename = "Network")]
    pub network: NetworkCapabilities,

    /// Security capabilities.
    #[serde(rename = "Security")]
    pub security: SecurityCapabilities,

    /// System capabilities.
    #[serde(rename = "System")]
    pub system: SystemCapabilities,

    /// Miscellaneous capabilities.
    #[serde(rename = "Misc", default, skip_serializing_if = "Option::is_none")]
    pub misc: Option<MiscCapabilities>,
}

/// Network capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetworkCapabilities {
    /// IP filtering support.
    #[serde(rename = "@IPFilter", default, skip_serializing_if = "Option::is_none")]
    pub ip_filter: Option<bool>,

    /// Zero configuration support.
    #[serde(
        rename = "@ZeroConfiguration",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub zero_configuration: Option<bool>,

    /// IPv6 support.
    #[serde(
        rename = "@IPVersion6",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ip_version6: Option<bool>,

    /// Dynamic DNS support.
    #[serde(rename = "@DynDNS", default, skip_serializing_if = "Option::is_none")]
    pub dyn_dns: Option<bool>,

    /// 802.11 configuration support.
    #[serde(
        rename = "@Dot11Configuration",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub dot11_configuration: Option<bool>,

    /// Number of 802.1X configurations supported.
    #[serde(
        rename = "@Dot1XConfigurations",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub dot1x_configurations: Option<i32>,

    /// Hostname from DHCP support.
    #[serde(
        rename = "@HostnameFromDHCP",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub hostname_from_dhcp: Option<bool>,

    /// Maximum NTP servers.
    #[serde(rename = "@NTP", default, skip_serializing_if = "Option::is_none")]
    pub ntp: Option<i32>,

    /// DHCPv6 support.
    #[serde(rename = "@DHCPv6", default, skip_serializing_if = "Option::is_none")]
    pub dhcpv6: Option<bool>,
}

/// Security capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SecurityCapabilities {
    /// TLS 1.0 support.
    #[serde(rename = "@TLS1.0", default, skip_serializing_if = "Option::is_none")]
    pub tls1_0: Option<bool>,

    /// TLS 1.1 support.
    #[serde(rename = "@TLS1.1", default, skip_serializing_if = "Option::is_none")]
    pub tls1_1: Option<bool>,

    /// TLS 1.2 support.
    #[serde(rename = "@TLS1.2", default, skip_serializing_if = "Option::is_none")]
    pub tls1_2: Option<bool>,

    /// Onboard key generation support.
    #[serde(
        rename = "@OnboardKeyGeneration",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub onboard_key_generation: Option<bool>,

    /// Access policy configuration support.
    #[serde(
        rename = "@AccessPolicyConfig",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub access_policy_config: Option<bool>,

    /// Default access policy support.
    #[serde(
        rename = "@DefaultAccessPolicy",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_access_policy: Option<bool>,

    /// 802.1X support.
    #[serde(rename = "@Dot1X", default, skip_serializing_if = "Option::is_none")]
    pub dot1x: Option<bool>,

    /// Remote user handling support.
    #[serde(
        rename = "@RemoteUserHandling",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub remote_user_handling: Option<bool>,

    /// X.509 token support.
    #[serde(
        rename = "@X.509Token",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub x509_token: Option<bool>,

    /// SAML token support.
    #[serde(
        rename = "@SAMLToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub saml_token: Option<bool>,

    /// Kerberos token support.
    #[serde(
        rename = "@KerberosToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub kerberos_token: Option<bool>,

    /// Username token support.
    #[serde(
        rename = "@UsernameToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub username_token: Option<bool>,

    /// HTTP digest support.
    #[serde(
        rename = "@HttpDigest",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub http_digest: Option<bool>,

    /// REL token support.
    #[serde(rename = "@RELToken", default, skip_serializing_if = "Option::is_none")]
    pub rel_token: Option<bool>,

    /// Maximum users.
    #[serde(rename = "@MaxUsers", default, skip_serializing_if = "Option::is_none")]
    pub max_users: Option<i32>,

    /// Maximum username length.
    #[serde(
        rename = "@MaxUserNameLength",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_user_name_length: Option<i32>,

    /// Maximum password length.
    #[serde(
        rename = "@MaxPasswordLength",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_password_length: Option<i32>,
}

/// System capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemCapabilities {
    /// Discovery resolve support.
    #[serde(
        rename = "@DiscoveryResolve",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub discovery_resolve: Option<bool>,

    /// Discovery bye support.
    #[serde(
        rename = "@DiscoveryBye",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub discovery_bye: Option<bool>,

    /// Remote discovery support.
    #[serde(
        rename = "@RemoteDiscovery",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub remote_discovery: Option<bool>,

    /// System backup support.
    #[serde(
        rename = "@SystemBackup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub system_backup: Option<bool>,

    /// System logging support.
    #[serde(
        rename = "@SystemLogging",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub system_logging: Option<bool>,

    /// Firmware upgrade support.
    #[serde(
        rename = "@FirmwareUpgrade",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub firmware_upgrade: Option<bool>,

    /// HTTP firmware upgrade support.
    #[serde(
        rename = "@HttpFirmwareUpgrade",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub http_firmware_upgrade: Option<bool>,

    /// HTTP system backup support.
    #[serde(
        rename = "@HttpSystemBackup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub http_system_backup: Option<bool>,

    /// HTTP system logging support.
    #[serde(
        rename = "@HttpSystemLogging",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub http_system_logging: Option<bool>,

    /// HTTP support information support.
    #[serde(
        rename = "@HttpSupportInformation",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub http_support_information: Option<bool>,

    /// Storage configuration support.
    #[serde(
        rename = "@StorageConfiguration",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub storage_configuration: Option<bool>,

    /// Maximum storage configurations.
    #[serde(
        rename = "@MaxStorageConfigurations",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_storage_configurations: Option<i32>,

    /// Geo location entries support.
    #[serde(
        rename = "@GeoLocationEntries",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub geo_location_entries: Option<i32>,
}

/// Miscellaneous capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MiscCapabilities {
    /// Auxiliary commands.
    #[serde(
        rename = "@AuxiliaryCommands",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub auxiliary_commands: Option<String>,
}

// ============================================================================
// GetCapabilities (Legacy)
// ============================================================================

/// Capability category enumeration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CapabilityCategory {
    All,
    Analytics,
    Device,
    Events,
    Imaging,
    Media,
    PTZ,
}

impl Default for CapabilityCategory {
    fn default() -> Self {
        Self::All
    }
}

/// GetCapabilities request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetCapabilities")]
pub struct GetCapabilities {
    /// Capability categories to retrieve.
    #[serde(rename = "Category", default)]
    pub category: Vec<CapabilityCategory>,
}

/// GetCapabilities response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetCapabilitiesResponse")]
pub struct GetCapabilitiesResponse {
    /// Device capabilities.
    #[serde(rename = "Capabilities")]
    pub capabilities: Capabilities,
}

/// Device capabilities (legacy format).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Capabilities {
    /// Analytics capabilities.
    #[serde(rename = "Analytics", default, skip_serializing_if = "Option::is_none")]
    pub analytics: Option<AnalyticsCapabilities>,

    /// Device capabilities.
    #[serde(rename = "Device", default, skip_serializing_if = "Option::is_none")]
    pub device: Option<DeviceCapabilities>,

    /// Events capabilities.
    #[serde(rename = "Events", default, skip_serializing_if = "Option::is_none")]
    pub events: Option<EventsCapabilities>,

    /// Imaging capabilities.
    #[serde(rename = "Imaging", default, skip_serializing_if = "Option::is_none")]
    pub imaging: Option<ImagingCapabilities>,

    /// Media capabilities.
    #[serde(rename = "Media", default, skip_serializing_if = "Option::is_none")]
    pub media: Option<MediaCapabilities>,

    /// PTZ capabilities.
    #[serde(rename = "PTZ", default, skip_serializing_if = "Option::is_none")]
    pub ptz: Option<PTZCapabilities>,

    /// Extension with DeviceIO and other capabilities.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<CapabilitiesExtension>,
}

/// Capabilities extension with DeviceIO.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CapabilitiesExtension {
    /// Device IO capabilities.
    #[serde(rename = "DeviceIO", default, skip_serializing_if = "Option::is_none")]
    pub device_io: Option<DeviceIOCapabilities>,
}

/// Device IO capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceIOCapabilities {
    /// Device IO service address.
    #[serde(rename = "XAddr")]
    pub x_addr: String,

    /// Number of video sources.
    #[serde(
        rename = "VideoSources",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub video_sources: Option<i32>,

    /// Number of video outputs.
    #[serde(
        rename = "VideoOutputs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub video_outputs: Option<i32>,

    /// Number of audio sources.
    #[serde(
        rename = "AudioSources",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub audio_sources: Option<i32>,

    /// Number of audio outputs.
    #[serde(
        rename = "AudioOutputs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub audio_outputs: Option<i32>,

    /// Number of relay outputs.
    #[serde(
        rename = "RelayOutputs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub relay_outputs: Option<i32>,
}

/// Analytics capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsCapabilities {
    /// Analytics service address.
    #[serde(rename = "XAddr")]
    pub x_addr: String,

    /// Rule support.
    #[serde(rename = "RuleSupport")]
    pub rule_support: bool,

    /// Analytics module support.
    #[serde(rename = "AnalyticsModuleSupport")]
    pub analytics_module_support: bool,
}

/// Device capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// Device service address.
    #[serde(rename = "XAddr")]
    pub x_addr: String,

    /// Network capabilities.
    #[serde(rename = "Network", default, skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkCapabilitiesLegacy>,

    /// System capabilities.
    #[serde(rename = "System", default, skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemCapabilitiesLegacy>,

    /// IO capabilities.
    #[serde(rename = "IO", default, skip_serializing_if = "Option::is_none")]
    pub io: Option<IOCapabilities>,

    /// Security capabilities.
    #[serde(rename = "Security", default, skip_serializing_if = "Option::is_none")]
    pub security: Option<SecurityCapabilitiesLegacy>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// Legacy network capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetworkCapabilitiesLegacy {
    /// IP filter support.
    #[serde(rename = "IPFilter", default, skip_serializing_if = "Option::is_none")]
    pub ip_filter: Option<bool>,

    /// Zero configuration support.
    #[serde(
        rename = "ZeroConfiguration",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub zero_configuration: Option<bool>,

    /// IPv6 support.
    #[serde(
        rename = "IPVersion6",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ip_version6: Option<bool>,

    /// Dynamic DNS support.
    #[serde(rename = "DynDNS", default, skip_serializing_if = "Option::is_none")]
    pub dyn_dns: Option<bool>,

    /// Extension containing Dot11Configuration and other network-specific extensions.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<NetworkCapabilitiesExtension>,
}

/// Network capabilities extension with Dot11Configuration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetworkCapabilitiesExtension {
    /// Dot11 (Wi-Fi) configuration support.
    #[serde(
        rename = "Dot11Configuration",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub dot11_configuration: Option<bool>,
}

/// Legacy system capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemCapabilitiesLegacy {
    /// Discovery resolve support.
    #[serde(rename = "DiscoveryResolve")]
    pub discovery_resolve: bool,

    /// Discovery bye support.
    #[serde(rename = "DiscoveryBye")]
    pub discovery_bye: bool,

    /// Remote discovery support.
    #[serde(rename = "RemoteDiscovery")]
    pub remote_discovery: bool,

    /// System backup support.
    #[serde(rename = "SystemBackup")]
    pub system_backup: bool,

    /// System logging support.
    #[serde(
        rename = "SystemLogging",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub system_logging: Option<bool>,

    /// Firmware upgrade support.
    #[serde(rename = "FirmwareUpgrade")]
    pub firmware_upgrade: bool,

    /// Supported ONVIF versions.
    #[serde(rename = "SupportedVersions", default)]
    pub supported_versions: Vec<OnvifVersion>,

    /// Extension with HTTP firmware/system capabilities.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<SystemCapabilitiesExtension>,
}

/// System capabilities extension with HTTP-based operations.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemCapabilitiesExtension {
    /// HTTP firmware upgrade support.
    #[serde(
        rename = "HttpFirmwareUpgrade",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub http_firmware_upgrade: Option<bool>,

    /// HTTP system backup support.
    #[serde(
        rename = "HttpSystemBackup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub http_system_backup: Option<bool>,

    /// HTTP system logging support.
    #[serde(
        rename = "HttpSystemLogging",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub http_system_logging: Option<bool>,

    /// HTTP support information.
    #[serde(
        rename = "HttpSupportInformation",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub http_support_information: Option<bool>,

    /// Further extension (nested).
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// IO capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IOCapabilities {
    /// Number of input connectors.
    #[serde(
        rename = "InputConnectors",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub input_connectors: Option<i32>,

    /// Number of relay outputs.
    #[serde(
        rename = "RelayOutputs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub relay_outputs: Option<i32>,

    /// Extension with Auxiliary support.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<IOCapabilitiesExtension>,
}

/// IO capabilities extension.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IOCapabilitiesExtension {
    /// Auxiliary output support.
    #[serde(rename = "Auxiliary", default, skip_serializing_if = "Option::is_none")]
    pub auxiliary: Option<bool>,

    /// Further extension (nested).
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// Legacy security capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SecurityCapabilitiesLegacy {
    /// TLS 1.1 support.
    #[serde(rename = "TLS1.1")]
    pub tls1_1: bool,

    /// TLS 1.2 support.
    #[serde(rename = "TLS1.2")]
    pub tls1_2: bool,

    /// Onboard key generation support.
    #[serde(rename = "OnboardKeyGeneration")]
    pub onboard_key_generation: bool,

    /// Access policy configuration support.
    #[serde(rename = "AccessPolicyConfig")]
    pub access_policy_config: bool,

    /// X.509 token support.
    #[serde(rename = "X.509Token")]
    pub x509_token: bool,

    /// SAML token support.
    #[serde(rename = "SAMLToken")]
    pub saml_token: bool,

    /// Kerberos token support.
    #[serde(rename = "KerberosToken")]
    pub kerberos_token: bool,

    /// REL token support.
    #[serde(rename = "RELToken")]
    pub rel_token: bool,

    /// Extension with TLS1.0 and other security extensions.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<SecurityCapabilitiesExtension>,
}

/// Security capabilities extension.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SecurityCapabilitiesExtension {
    /// TLS 1.0 support.
    #[serde(rename = "TLS1.0", default, skip_serializing_if = "Option::is_none")]
    pub tls1_0: Option<bool>,

    /// Further extension with Dot1X.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<SecurityCapabilitiesExtension2>,
}

/// Second-level security capabilities extension.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SecurityCapabilitiesExtension2 {
    /// IEEE 802.1X support.
    #[serde(rename = "Dot1X", default, skip_serializing_if = "Option::is_none")]
    pub dot1x: Option<bool>,

    /// Supported EAP methods.
    #[serde(
        rename = "SupportedEAPMethod",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub supported_eap_method: Option<i32>,

    /// Remote user handling support.
    #[serde(
        rename = "RemoteUserHandling",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub remote_user_handling: Option<bool>,
}

/// Events capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventsCapabilities {
    /// Events service address.
    #[serde(rename = "XAddr")]
    pub x_addr: String,

    /// WS-Subscription policy support.
    #[serde(rename = "WSSubscriptionPolicySupport")]
    pub ws_subscription_policy_support: bool,

    /// WS-Pull point support.
    #[serde(rename = "WSPullPointSupport")]
    pub ws_pull_point_support: bool,

    /// WS-Pausable subscription manager support.
    #[serde(rename = "WSPausableSubscriptionManagerInterfaceSupport")]
    pub ws_pausable_subscription_manager_interface_support: bool,
}

/// Imaging capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImagingCapabilities {
    /// Imaging service address.
    #[serde(rename = "XAddr")]
    pub x_addr: String,
}

/// Media capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaCapabilities {
    /// Media service address.
    #[serde(rename = "XAddr")]
    pub x_addr: String,

    /// Streaming capabilities.
    #[serde(
        rename = "StreamingCapabilities",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub streaming_capabilities: Option<RealTimeStreamingCapabilities>,

    /// Extension with profile capabilities.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<MediaCapabilitiesExtension>,
}

/// Media capabilities extension.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MediaCapabilitiesExtension {
    /// Profile capabilities.
    #[serde(
        rename = "ProfileCapabilities",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub profile_capabilities: Option<ProfileCapabilities>,
}

/// Profile capabilities within media extension.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProfileCapabilities {
    /// Maximum number of profiles.
    #[serde(
        rename = "MaximumNumberOfProfiles",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub maximum_number_of_profiles: Option<i32>,
}

/// Real-time streaming capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RealTimeStreamingCapabilities {
    /// RTP multicast support.
    #[serde(
        rename = "RTPMulticast",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub rtp_multicast: Option<bool>,

    /// RTP over TCP support.
    #[serde(rename = "RTP_TCP", default, skip_serializing_if = "Option::is_none")]
    pub rtp_tcp: Option<bool>,

    /// RTP/RTSP/TCP support.
    #[serde(
        rename = "RTP_RTSP_TCP",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub rtp_rtsp_tcp: Option<bool>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// PTZ capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PTZCapabilities {
    /// PTZ service address.
    #[serde(rename = "XAddr")]
    pub x_addr: String,
}

// ============================================================================
// System Date and Time
// ============================================================================

/// GetSystemDateAndTime request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetSystemDateAndTime")]
pub struct GetSystemDateAndTime {}

/// GetSystemDateAndTime response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetSystemDateAndTimeResponse")]
pub struct GetSystemDateAndTimeResponse {
    /// System date and time information.
    #[serde(rename = "SystemDateAndTime")]
    pub system_date_and_time: SystemDateTime,
}

/// SetSystemDateAndTime request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetSystemDateAndTime")]
pub struct SetSystemDateAndTime {
    /// Date/time type (Manual or NTP).
    #[serde(rename = "DateTimeType")]
    pub date_time_type: SetDateTimeType,

    /// Daylight savings enabled.
    #[serde(rename = "DaylightSavings")]
    pub daylight_savings: bool,

    /// Timezone (optional).
    #[serde(rename = "TimeZone", default, skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<TimeZone>,

    /// UTC date/time (optional, for manual setting).
    #[serde(
        rename = "UTCDateTime",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub utc_date_time: Option<DateTime>,
}

/// SetSystemDateAndTime response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetSystemDateAndTimeResponse")]
pub struct SetSystemDateAndTimeResponse {}

// ============================================================================
// System Reboot
// ============================================================================

/// SystemReboot request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SystemReboot")]
pub struct SystemReboot {}

/// SystemReboot response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SystemRebootResponse")]
pub struct SystemRebootResponse {
    /// Message indicating reboot status.
    #[serde(rename = "Message")]
    pub message: String,
}

impl Default for SystemRebootResponse {
    fn default() -> Self {
        Self {
            message: "Rebooting".to_string(),
        }
    }
}

// ============================================================================
// Factory Default
// ============================================================================

/// SetSystemFactoryDefault request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetSystemFactoryDefault")]
pub struct SetSystemFactoryDefault {
    /// Factory default type (Hard or Soft).
    #[serde(rename = "FactoryDefault")]
    pub factory_default: FactoryDefaultType,
}

/// SetSystemFactoryDefault response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetSystemFactoryDefaultResponse")]
pub struct SetSystemFactoryDefaultResponse {}

// ============================================================================
// Hostname
// ============================================================================

/// GetHostname request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetHostname")]
pub struct GetHostname {}

/// GetHostname response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetHostnameResponse")]
pub struct GetHostnameResponse {
    /// Hostname information.
    #[serde(rename = "HostnameInformation")]
    pub hostname_information: HostnameInformation,
}

/// Hostname information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostnameInformation {
    /// Whether hostname is obtained from DHCP.
    #[serde(rename = "FromDHCP")]
    pub from_dhcp: bool,

    /// Hostname value.
    #[serde(rename = "Name", default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// SetHostname request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetHostname")]
pub struct SetHostname {
    /// New hostname.
    #[serde(rename = "Name")]
    pub name: String,
}

/// SetHostname response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetHostnameResponse")]
pub struct SetHostnameResponse {}

// ============================================================================
// Network Interfaces
// ============================================================================

/// GetNetworkInterfaces request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetNetworkInterfaces")]
pub struct GetNetworkInterfaces {}

/// GetNetworkInterfaces response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetNetworkInterfacesResponse")]
pub struct GetNetworkInterfacesResponse {
    /// List of network interfaces.
    #[serde(rename = "NetworkInterfaces", default)]
    pub network_interfaces: Vec<NetworkInterface>,
}

/// Network interface information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Interface token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// Interface enabled.
    #[serde(rename = "Enabled")]
    pub enabled: bool,

    /// Interface information.
    #[serde(rename = "Info", default, skip_serializing_if = "Option::is_none")]
    pub info: Option<NetworkInterfaceInfo>,

    /// Link configuration.
    #[serde(rename = "Link", default, skip_serializing_if = "Option::is_none")]
    pub link: Option<NetworkInterfaceLink>,

    /// IPv4 configuration.
    #[serde(rename = "IPv4", default, skip_serializing_if = "Option::is_none")]
    pub ipv4: Option<IPv4NetworkInterface>,

    /// IPv6 configuration.
    #[serde(rename = "IPv6", default, skip_serializing_if = "Option::is_none")]
    pub ipv6: Option<IPv6NetworkInterface>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// Network interface information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkInterfaceInfo {
    /// Interface name.
    #[serde(rename = "Name", default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Hardware address (MAC).
    #[serde(rename = "HwAddress")]
    pub hw_address: String,

    /// MTU.
    #[serde(rename = "MTU", default, skip_serializing_if = "Option::is_none")]
    pub mtu: Option<i32>,
}

/// Network interface link configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkInterfaceLink {
    /// Admin settings.
    #[serde(rename = "AdminSettings")]
    pub admin_settings: NetworkInterfaceConnectionSetting,

    /// Operational settings.
    #[serde(rename = "OperSettings")]
    pub oper_settings: NetworkInterfaceConnectionSetting,

    /// Interface up/down status.
    #[serde(rename = "InterfaceType")]
    pub interface_type: i32,
}

/// Network interface connection settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkInterfaceConnectionSetting {
    /// Auto-negotiate.
    #[serde(rename = "AutoNegotiation")]
    pub auto_negotiation: bool,

    /// Speed in Mbps.
    #[serde(rename = "Speed")]
    pub speed: i32,

    /// Duplex mode.
    #[serde(rename = "Duplex")]
    pub duplex: Duplex,
}

/// Duplex mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Duplex {
    Full,
    Half,
}

impl Default for Duplex {
    fn default() -> Self {
        Self::Full
    }
}

/// IPv4 network interface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IPv4NetworkInterface {
    /// IPv4 enabled.
    #[serde(rename = "Enabled")]
    pub enabled: bool,

    /// IPv4 configuration.
    #[serde(rename = "Config")]
    pub config: IPv4Configuration,
}

/// IPv4 configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IPv4Configuration {
    /// Manual addresses.
    #[serde(rename = "Manual", default)]
    pub manual: Vec<PrefixedIPv4Address>,

    /// Link-local address.
    #[serde(rename = "LinkLocal", default, skip_serializing_if = "Option::is_none")]
    pub link_local: Option<PrefixedIPv4Address>,

    /// Address from DHCP.
    #[serde(rename = "FromDHCP", default, skip_serializing_if = "Option::is_none")]
    pub from_dhcp: Option<PrefixedIPv4Address>,

    /// DHCP enabled.
    #[serde(rename = "DHCP")]
    pub dhcp: bool,
}

/// Prefixed IPv4 address.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefixedIPv4Address {
    /// IPv4 address.
    #[serde(rename = "Address")]
    pub address: String,

    /// Prefix length.
    #[serde(rename = "PrefixLength")]
    pub prefix_length: i32,
}

/// IPv6 network interface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IPv6NetworkInterface {
    /// IPv6 enabled.
    #[serde(rename = "Enabled")]
    pub enabled: bool,

    /// IPv6 configuration.
    #[serde(rename = "Config", default, skip_serializing_if = "Option::is_none")]
    pub config: Option<IPv6Configuration>,
}

/// IPv6 configuration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IPv6Configuration {
    /// Accept router advertisements.
    #[serde(
        rename = "AcceptRouterAdvert",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub accept_router_advert: Option<bool>,

    /// DHCP configuration.
    #[serde(rename = "DHCP")]
    pub dhcp: IPv6DHCPConfiguration,

    /// Manual addresses.
    #[serde(rename = "Manual", default)]
    pub manual: Vec<PrefixedIPv6Address>,

    /// Link-local addresses.
    #[serde(rename = "LinkLocal", default)]
    pub link_local: Vec<PrefixedIPv6Address>,

    /// Addresses from DHCP.
    #[serde(rename = "FromDHCP", default)]
    pub from_dhcp: Vec<PrefixedIPv6Address>,

    /// Addresses from router advertisement.
    #[serde(rename = "FromRA", default)]
    pub from_ra: Vec<PrefixedIPv6Address>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// IPv6 DHCP configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IPv6DHCPConfiguration {
    Auto,
    Stateful,
    Stateless,
    Off,
}

impl Default for IPv6DHCPConfiguration {
    fn default() -> Self {
        Self::Off
    }
}

/// Prefixed IPv6 address.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefixedIPv6Address {
    /// IPv6 address.
    #[serde(rename = "Address")]
    pub address: String,

    /// Prefix length.
    #[serde(rename = "PrefixLength")]
    pub prefix_length: i32,
}

// ============================================================================
// Scopes
// ============================================================================

/// GetScopes request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetScopes")]
pub struct GetScopes {}

/// GetScopes response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetScopesResponse")]
pub struct GetScopesResponse {
    /// List of scopes.
    #[serde(rename = "Scopes", default)]
    pub scopes: Vec<Scope>,
}

/// SetScopes request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetScopes")]
pub struct SetScopes {
    /// Scopes to set.
    #[serde(rename = "Scopes", default)]
    pub scopes: Vec<String>,
}

/// SetScopes response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetScopesResponse")]
pub struct SetScopesResponse {}

/// AddScopes request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "AddScopes")]
pub struct AddScopes {
    /// Scopes to add.
    #[serde(rename = "ScopeItem", default)]
    pub scope_item: Vec<String>,
}

/// AddScopes response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "AddScopesResponse")]
pub struct AddScopesResponse {}

/// RemoveScopes request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "RemoveScopes")]
pub struct RemoveScopes {
    /// Scopes to remove.
    #[serde(rename = "ScopeItem", default)]
    pub scope_item: Vec<String>,
}

/// RemoveScopes response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "RemoveScopesResponse")]
pub struct RemoveScopesResponse {
    /// Remaining scopes.
    #[serde(rename = "ScopeItem", default)]
    pub scope_item: Vec<String>,
}

// ============================================================================
// Discovery Mode
// ============================================================================

/// GetDiscoveryMode request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetDiscoveryMode")]
pub struct GetDiscoveryMode {}

/// GetDiscoveryMode response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetDiscoveryModeResponse")]
pub struct GetDiscoveryModeResponse {
    /// Current discovery mode.
    #[serde(rename = "DiscoveryMode")]
    pub discovery_mode: DiscoveryMode,
}

impl Default for GetDiscoveryModeResponse {
    fn default() -> Self {
        Self {
            discovery_mode: DiscoveryMode::Discoverable,
        }
    }
}

/// SetDiscoveryMode request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetDiscoveryMode")]
pub struct SetDiscoveryMode {
    /// New discovery mode.
    #[serde(rename = "DiscoveryMode")]
    pub discovery_mode: DiscoveryMode,
}

/// SetDiscoveryMode response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetDiscoveryModeResponse")]
pub struct SetDiscoveryModeResponse {}

// ============================================================================
// Users
// ============================================================================

/// GetUsers request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetUsers")]
pub struct GetUsers {}

/// GetUsers response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetUsersResponse")]
pub struct GetUsersResponse {
    /// List of users.
    #[serde(rename = "User", default)]
    pub users: Vec<User>,
}

/// CreateUsers request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "CreateUsers")]
pub struct CreateUsers {
    /// Users to create.
    #[serde(rename = "User", default)]
    pub users: Vec<User>,
}

/// CreateUsers response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "CreateUsersResponse")]
pub struct CreateUsersResponse {}

/// DeleteUsers request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "DeleteUsers")]
pub struct DeleteUsers {
    /// Usernames to delete.
    #[serde(rename = "Username", default)]
    pub usernames: Vec<String>,
}

/// DeleteUsers response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "DeleteUsersResponse")]
pub struct DeleteUsersResponse {}

/// SetUser request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetUser")]
pub struct SetUser {
    /// Users to update.
    #[serde(rename = "User", default)]
    pub users: Vec<User>,
}

/// SetUser response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetUserResponse")]
pub struct SetUserResponse {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_device_information_response_default() {
        let response = GetDeviceInformationResponse::default();
        assert_eq!(response.manufacturer, "ONVIF");
        assert_eq!(response.model, "Unknown");
    }

    #[test]
    fn test_capability_category_default() {
        let category = CapabilityCategory::default();
        assert_eq!(category, CapabilityCategory::All);
    }

    #[test]
    fn test_discovery_mode_response_default() {
        let response = GetDiscoveryModeResponse::default();
        assert_eq!(response.discovery_mode, DiscoveryMode::Discoverable);
    }

    #[test]
    fn test_system_reboot_response_default() {
        let response = SystemRebootResponse::default();
        assert_eq!(response.message, "Rebooting");
    }
}
