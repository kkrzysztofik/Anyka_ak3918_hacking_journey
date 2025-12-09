//! Device Service type re-exports and extensions.
//!
//! This module re-exports WSDL-generated types from `crate::onvif::types::device`
//! and provides any implementation-specific additions or conversions needed
//! for the Device Service handlers.
//!
//! All types in this module are validated against `devicemgmt.wsdl` definitions.

// Re-export all device types from the WSDL-generated module
pub use crate::onvif::types::device::*;

// Re-export common types needed by device operations
pub use crate::onvif::types::common::{
    DateTime, DiscoveryMode, FactoryDefaultType, OnvifVersion, ReferenceToken, Scope,
    SetDateTimeType, SystemDateTime, TimeZone, User, UserLevel,
};

// ============================================================================
// Service Information
// ============================================================================

/// ONVIF Device Service namespace URI.
pub const DEVICE_SERVICE_NAMESPACE: &str = "http://www.onvif.org/ver10/device/wsdl";

/// ONVIF Media Service namespace URI.
pub const MEDIA_SERVICE_NAMESPACE: &str = "http://www.onvif.org/ver10/media/wsdl";

/// ONVIF PTZ Service namespace URI.
pub const PTZ_SERVICE_NAMESPACE: &str = "http://www.onvif.org/ver20/ptz/wsdl";

/// ONVIF Imaging Service namespace URI.
pub const IMAGING_SERVICE_NAMESPACE: &str = "http://www.onvif.org/ver20/imaging/wsdl";

/// Supported ONVIF version.
pub const ONVIF_VERSION: OnvifVersion = OnvifVersion { major: 2, minor: 5 };

// ============================================================================
// Builder Helpers
// ============================================================================

/// Create a Service entry for GetServices response.
pub fn create_service(namespace: &str, x_addr: &str, version: OnvifVersion) -> Service {
    Service {
        namespace: namespace.to_string(),
        x_addr: x_addr.to_string(),
        capabilities: None,
        version,
    }
}

/// Build device capabilities (legacy format for GetCapabilities).
pub fn build_device_capabilities(base_url: &str) -> DeviceCapabilities {
    DeviceCapabilities {
        x_addr: format!("{}/onvif/device_service", base_url),
        network: Some(NetworkCapabilitiesLegacy {
            ip_filter: Some(false),
            zero_configuration: Some(false),
            ip_version6: Some(false),
            dyn_dns: Some(false),
            extension: None,
        }),
        system: Some(SystemCapabilitiesLegacy {
            discovery_resolve: true,
            discovery_bye: true,
            remote_discovery: false,
            system_backup: false,
            system_logging: Some(false),
            firmware_upgrade: false,
            supported_versions: vec![ONVIF_VERSION],
            extension: None,
        }),
        io: None,
        security: Some(SecurityCapabilitiesLegacy {
            tls1_1: false,
            tls1_2: false,
            onboard_key_generation: false,
            access_policy_config: false,
            x509_token: false,
            saml_token: false,
            kerberos_token: false,
            rel_token: false,
            extension: Some(SecurityCapabilitiesExtension {
                tls1_0: Some(false),
                extension: None,
            }),
        }),
        extension: None,
    }
}

/// Build media capabilities (legacy format for GetCapabilities).
pub fn build_media_capabilities(base_url: &str) -> MediaCapabilities {
    MediaCapabilities {
        x_addr: format!("{}/onvif/media_service", base_url),
        streaming_capabilities: Some(RealTimeStreamingCapabilities {
            rtp_multicast: Some(false),
            rtp_tcp: Some(true),
            rtp_rtsp_tcp: Some(true),
            extension: None,
        }),
        extension: Some(MediaCapabilitiesExtension {
            profile_capabilities: Some(ProfileCapabilities {
                maximum_number_of_profiles: Some(4),
            }),
        }),
    }
}

/// Build PTZ capabilities (legacy format for GetCapabilities).
pub fn build_ptz_capabilities(base_url: &str) -> PTZCapabilities {
    PTZCapabilities {
        x_addr: format!("{}/onvif/ptz_service", base_url),
    }
}

/// Build imaging capabilities (legacy format for GetCapabilities).
pub fn build_imaging_capabilities(base_url: &str) -> ImagingCapabilities {
    ImagingCapabilities {
        x_addr: format!("{}/onvif/imaging_service", base_url),
    }
}

/// Build Device Service capabilities response.
pub fn build_device_service_capabilities() -> DeviceServiceCapabilities {
    DeviceServiceCapabilities {
        network: NetworkCapabilities {
            ip_filter: Some(false),
            zero_configuration: Some(false),
            ip_version6: Some(false),
            dyn_dns: Some(false),
            dot11_configuration: Some(false),
            dot1x_configurations: Some(0),
            hostname_from_dhcp: Some(true),
            ntp: Some(1),
            dhcpv6: Some(false),
        },
        security: SecurityCapabilities {
            tls1_0: Some(false),
            tls1_1: Some(false),
            tls1_2: Some(false),
            onboard_key_generation: Some(false),
            access_policy_config: Some(false),
            default_access_policy: Some(false),
            dot1x: Some(false),
            remote_user_handling: Some(false),
            x509_token: Some(false),
            saml_token: Some(false),
            kerberos_token: Some(false),
            username_token: Some(true),
            http_digest: Some(true),
            rel_token: Some(false),
            max_users: Some(8),
            max_user_name_length: Some(64),
            max_password_length: Some(64),
        },
        system: SystemCapabilities {
            discovery_resolve: Some(true),
            discovery_bye: Some(true),
            remote_discovery: Some(false),
            system_backup: Some(false),
            system_logging: Some(false),
            firmware_upgrade: Some(false),
            http_firmware_upgrade: Some(false),
            http_system_backup: Some(false),
            http_system_logging: Some(false),
            http_support_information: Some(false),
            storage_configuration: Some(false),
            max_storage_configurations: None,
            geo_location_entries: None,
        },
        misc: None,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_service() {
        let service = create_service(
            DEVICE_SERVICE_NAMESPACE,
            "http://192.168.1.100:8080/onvif/device_service",
            ONVIF_VERSION,
        );

        assert_eq!(service.namespace, DEVICE_SERVICE_NAMESPACE);
        assert!(service.x_addr.contains("device_service"));
        assert_eq!(service.version.major, 2);
        assert_eq!(service.version.minor, 5);
    }

    #[test]
    fn test_build_device_capabilities() {
        let caps = build_device_capabilities("http://192.168.1.100:8080");

        assert!(caps.x_addr.contains("device_service"));
        assert!(caps.network.is_some());
        assert!(caps.system.is_some());
        assert!(caps.security.is_some());
    }

    #[test]
    fn test_build_device_service_capabilities() {
        let caps = build_device_service_capabilities();

        assert_eq!(caps.security.max_users, Some(8));
        assert_eq!(caps.security.username_token, Some(true));
        assert_eq!(caps.security.http_digest, Some(true));
        assert_eq!(caps.system.discovery_bye, Some(true));
    }
}
