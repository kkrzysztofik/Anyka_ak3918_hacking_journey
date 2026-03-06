//! Device Service implementation.
//!
//! This module contains the DeviceService struct and the ServiceHandler trait implementation.

use std::sync::Arc;

use crate::config::{ConfigRuntime, PasswordManager, UserStorage};
use crate::onvif::dispatcher::{ServiceHandler, parse_body};
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::common::{DiscoveryMode, Scope};
use crate::onvif::types::device::*;
use crate::platform::Platform;

use async_trait::async_trait;

pub use super::ops::discovery;
pub use super::ops::network;
pub use super::ops::system;
pub use super::ops::users;

// Re-export ops modules for external use
pub use super::ops::{
    discovery as discovery_ops, network as network_ops, system as system_ops, users as users_ops,
};

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
    pub(crate) users: Arc<UserStorage>,
    /// Configuration runtime.
    pub(crate) config: Arc<ConfigRuntime>,
    /// Platform abstraction (optional for backward compatibility).
    pub(crate) platform: Option<Arc<dyn Platform>>,
    /// Scopes storage (in-memory for now).
    pub(crate) scopes: parking_lot::RwLock<Vec<Scope>>,
    /// Discovery mode.
    pub(crate) discovery_mode: parking_lot::RwLock<DiscoveryMode>,
}

impl DeviceService {
    /// Create a new Device Service.
    pub fn new(users: Arc<UserStorage>, _password_manager: Arc<PasswordManager>) -> Self {
        Self {
            users,
            config: Arc::new(ConfigRuntime::new(Default::default())),
            platform: None,
            scopes: parking_lot::RwLock::new(discovery_ops::default_scopes()),
            discovery_mode: parking_lot::RwLock::new(DiscoveryMode::Discoverable),
        }
    }

    /// Create a new Device Service with configuration and platform.
    pub fn with_config_and_platform(
        users: Arc<UserStorage>,
        _password_manager: Arc<PasswordManager>,
        config: Arc<ConfigRuntime>,
        platform: Arc<dyn Platform>,
    ) -> Self {
        Self {
            users,
            config,
            platform: Some(platform),
            scopes: parking_lot::RwLock::new(discovery_ops::default_scopes()),
            discovery_mode: parking_lot::RwLock::new(DiscoveryMode::Discoverable),
        }
    }

    /// Get the base URL for service addresses.
    /// Uses detected IP address for proper XAddr values in capabilities.
    #[allow(dead_code)]
    pub(crate) fn base_url(&self) -> String {
        // Use canonical external_ip helper for consistency across all ONVIF services
        let address = crate::platform::external_ip(&self.config);
        let port = self.config.read().server.port;
        format!("http://{}:{}", address, port)
    }
}

// ========================================================================
// ServiceHandler Implementation for DeviceService
// ========================================================================

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
                let _request: GetDeviceInformation = parse_body(body_xml)?;
                let response =
                    system_ops::handle_get_device_information(&self.platform, &self.config).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Capabilities Operations
            "GetCapabilities" => {
                let request: GetCapabilities = parse_body(body_xml)?;
                let response = system_ops::handle_get_capabilities(&self.config, request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetServices" => {
                let request: GetServices = parse_body(body_xml)?;
                let response = system_ops::handle_get_services(&self.config, request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetServiceCapabilities" => {
                let request: GetServiceCapabilities = parse_body(body_xml)?;
                let response = system_ops::handle_get_service_capabilities(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // System Date/Time Operations
            "GetSystemDateAndTime" => {
                let request: GetSystemDateAndTime = parse_body(body_xml)?;
                let response = system_ops::handle_get_system_date_and_time(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetSystemDateAndTime" => {
                let request: SetSystemDateAndTime = parse_body(body_xml)?;
                let response = system_ops::handle_set_system_date_and_time(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // System Operations
            "SystemReboot" => {
                let request: SystemReboot = parse_body(body_xml)?;
                let response = system_ops::handle_system_reboot(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetSystemFactoryDefault" => {
                let request: SetSystemFactoryDefault = parse_body(body_xml)?;
                let response = system_ops::handle_set_system_factory_default(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Certificate Operations
            "GetCertificates" => {
                let request: GetCertificates = parse_body(body_xml)?;
                let response = system_ops::handle_get_certificates(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetCertificatesStatus" => {
                let request: GetCertificatesStatus = parse_body(body_xml)?;
                let response = system_ops::handle_get_certificates_status(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "CreateCertificate" => {
                let request: CreateCertificate = parse_body(body_xml)?;
                let response = system_ops::handle_create_certificate(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "LoadCertificates" => {
                let request: LoadCertificates = parse_body(body_xml)?;
                let response = system_ops::handle_load_certificates(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "DeleteCertificates" => {
                let request: DeleteCertificates = parse_body(body_xml)?;
                let response = system_ops::handle_delete_certificates(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Backup/Restore Operations
            "GetSystemBackup" => {
                let request: GetSystemBackup = parse_body(body_xml)?;
                let response = system_ops::handle_get_system_backup(&self.config, request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "RestoreSystem" => {
                let request: RestoreSystem = parse_body(body_xml)?;
                let response = system_ops::handle_restore_system(&self.config, request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Relay Operations
            "GetRelayOutputs" => {
                let request: GetRelayOutputs = parse_body(body_xml)?;
                let response = system_ops::handle_get_relay_outputs(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Hostname Operations
            "GetHostname" => {
                let request: GetHostname = parse_body(body_xml)?;
                let response = network_ops::handle_get_hostname(&self.config, request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetHostname" => {
                let request: SetHostname = parse_body(body_xml)?;
                let response = network_ops::handle_set_hostname(&self.config, request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Network Operations
            "GetNetworkInterfaces" => {
                let request: GetNetworkInterfaces = parse_body(body_xml)?;
                let response = network_ops::handle_get_network_interfaces(
                    &self.platform,
                    &self.config,
                    request,
                )
                .await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetNetworkDefaultGateway" => {
                let request: GetNetworkDefaultGateway = parse_body(body_xml)?;
                let response =
                    network_ops::handle_get_network_default_gateway(&self.config, request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // DNS Operations
            "GetDNS" => {
                let request: GetDNS = parse_body(body_xml)?;
                let response = network_ops::handle_get_dns(&self.platform, request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetDNS" => {
                let request: SetDNS = parse_body(body_xml)?;
                let response = network_ops::handle_set_dns(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // NTP Operations
            "GetNTP" => {
                let request: GetNTP = parse_body(body_xml)?;
                let response = network_ops::handle_get_ntp(&self.platform, request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetNTP" => {
                let request: SetNTP = parse_body(body_xml)?;
                let response = network_ops::handle_set_ntp(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Network Protocol Operations
            "GetNetworkProtocols" => {
                let request: GetNetworkProtocols = parse_body(body_xml)?;
                let response = network_ops::handle_get_network_protocols(
                    &self.platform,
                    &self.config,
                    request,
                )
                .await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetNetworkProtocols" => {
                let request: SetNetworkProtocols = parse_body(body_xml)?;
                let response = network_ops::handle_set_network_protocols(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Scope Operations
            "GetScopes" => {
                let request: GetScopes = parse_body(body_xml)?;
                let response = discovery_ops::handle_get_scopes(&self.scopes, request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetScopes" => {
                let request: SetScopes = parse_body(body_xml)?;
                let response = discovery_ops::handle_set_scopes(&self.scopes, request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "AddScopes" => {
                let request: AddScopes = parse_body(body_xml)?;
                let response = discovery_ops::handle_add_scopes(&self.scopes, request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "RemoveScopes" => {
                let request: RemoveScopes = parse_body(body_xml)?;
                let response = discovery_ops::handle_remove_scopes(&self.scopes, request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Discovery Operations
            "GetDiscoveryMode" => {
                let request: GetDiscoveryMode = parse_body(body_xml)?;
                let response =
                    discovery_ops::handle_get_discovery_mode(&self.discovery_mode, request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetDiscoveryMode" => {
                let request: SetDiscoveryMode = parse_body(body_xml)?;
                let response =
                    discovery_ops::handle_set_discovery_mode(&self.discovery_mode, request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // User Management Operations
            "GetUsers" => {
                let request: GetUsers = parse_body(body_xml)?;
                let response = users_ops::handle_get_users(&self.users, request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "CreateUsers" => {
                // Note: Caller level should be extracted from auth context
                // For now, assume admin level for testing
                let request: CreateUsers = parse_body(body_xml)?;
                let response = users_ops::handle_create_users(
                    &self.users,
                    request,
                    crate::config::UserLevel::Administrator,
                )?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "DeleteUsers" => {
                // Note: Caller level should be extracted from auth context
                let request: DeleteUsers = parse_body(body_xml)?;
                let response = users_ops::handle_delete_users(
                    &self.users,
                    request,
                    crate::config::UserLevel::Administrator,
                )?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetUser" => {
                // Note: Caller level should be extracted from auth context
                let request: SetUser = parse_body(body_xml)?;
                let response = users_ops::handle_set_user(
                    &self.users,
                    request,
                    crate::config::UserLevel::Administrator,
                )?;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{PasswordManager, UserLevel, UserStorage};
    use std::sync::Arc;

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
    // ServiceHandler Error Path Tests
    // ========================================================================

    #[tokio::test]
    async fn test_service_handler_unknown_action_device() {
        let service = create_test_service();
        let result = service.handle_operation("UnknownAction", "<test/>").await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_service_handler_invalid_xml() {
        let service = create_test_service();
        let result = service
            .handle_operation("GetDeviceInformation", "<InvalidXml><Broken")
            .await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::WellFormed(_))));
    }

    #[tokio::test]
    async fn test_service_handler_get_device_information_xml() {
        let service = create_test_service();
        let xml = r#"<GetDeviceInformation/>"#;
        let result = service.handle_operation("GetDeviceInformation", xml).await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("GetDeviceInformationResponse"));
    }

    #[tokio::test]
    async fn test_service_handler_get_capabilities_xml() {
        let service = create_test_service();
        // GetCapabilities with empty category list (default)
        let xml = r#"<GetCapabilities/>"#;
        let result = service.handle_operation("GetCapabilities", xml).await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("GetCapabilitiesResponse"));
    }

    // ========================================================================
    // Base URL and IP Detection Tests
    // ========================================================================

    #[test]
    fn test_base_url_uses_detected_ip() {
        let users = Arc::new(UserStorage::new());
        users
            .create_user("admin", "admin123", UserLevel::Administrator)
            .unwrap();
        let password_manager = Arc::new(PasswordManager::new());
        let config = ConfigRuntime::new(Default::default());
        {
            let mut c = config.write();
            c.network.detected_ip = "192.168.1.100".to_string();
            c.network.mac_address = "AA:BB:CC:DD:EE:FF".to_string();
            c.network.dhcp_enabled = false;
            c.server.port = 8080;
        }

        let platform = Arc::new(crate::platform::StubPlatform::new());
        let service = DeviceService::with_config_and_platform(
            users,
            password_manager,
            Arc::new(config),
            platform,
        );

        // Access base_url through a handler that uses it
        let response = system_ops::handle_get_capabilities(
            &service.config,
            GetCapabilities { category: vec![] },
        )
        .unwrap();

        // Verify the URL contains the detected IP
        let device_caps = response.capabilities.device.unwrap();
        assert!(device_caps.x_addr.contains("192.168.1.100"));
        assert!(device_caps.x_addr.contains("8080"));
    }

    #[test]
    fn test_base_url_uses_static_ip_address() {
        let users = Arc::new(UserStorage::new());
        users
            .create_user("admin", "admin123", UserLevel::Administrator)
            .unwrap();
        let password_manager = Arc::new(PasswordManager::new());
        let config = ConfigRuntime::new(Default::default());
        {
            let mut c = config.write();
            // Static IP takes highest precedence
            c.network.ip_address = "192.168.1.50".to_string();
            // These should be ignored when ip_address is set
            c.network.detected_ip = "192.168.1.100".to_string();
            c.server.address = "10.0.0.1".to_string();
            c.server.port = 8080;
        }

        let platform = Arc::new(crate::platform::StubPlatform::new());
        let service = DeviceService::with_config_and_platform(
            users,
            password_manager,
            Arc::new(config),
            platform,
        );

        // Access base_url through a handler that uses it
        let response = system_ops::handle_get_capabilities(
            &service.config,
            GetCapabilities { category: vec![] },
        )
        .unwrap();

        // Verify the URL contains the static IP (not detected_ip or server.address)
        let device_caps = response.capabilities.device.unwrap();
        assert!(device_caps.x_addr.contains("192.168.1.50"));
        assert!(device_caps.x_addr.contains("8080"));
    }

    #[test]
    fn test_base_url_fallback_to_ip_address() {
        let users = Arc::new(UserStorage::new());
        users
            .create_user("admin", "admin123", UserLevel::Administrator)
            .unwrap();
        let password_manager = Arc::new(PasswordManager::new());
        let config = ConfigRuntime::new(Default::default());
        {
            let mut c = config.write();
            // Use server.address as per canonical external_ip precedence
            c.server.address = "10.0.0.1".to_string();
            c.server.port = 9000;
        }

        let platform = Arc::new(crate::platform::StubPlatform::new());
        let service = DeviceService::with_config_and_platform(
            users,
            password_manager,
            Arc::new(config),
            platform,
        );

        let response = system_ops::handle_get_capabilities(
            &service.config,
            GetCapabilities { category: vec![] },
        )
        .unwrap();

        let device_caps = response.capabilities.device.unwrap();
        assert!(device_caps.x_addr.contains("10.0.0.1"));
        assert!(device_caps.x_addr.contains("9000"));
    }

    #[test]
    fn test_base_url_fallback_to_127_0_0_1() {
        let users = Arc::new(UserStorage::new());
        users
            .create_user("admin", "admin123", UserLevel::Administrator)
            .unwrap();
        let password_manager = Arc::new(PasswordManager::new());
        let config = ConfigRuntime::new(Default::default());
        // No IP config, should fallback to 127.0.0.1

        let platform = Arc::new(crate::platform::StubPlatform::new());
        let service = DeviceService::with_config_and_platform(
            users,
            password_manager,
            Arc::new(config),
            platform,
        );

        let response = system_ops::handle_get_capabilities(
            &service.config,
            GetCapabilities { category: vec![] },
        )
        .unwrap();

        let device_caps = response.capabilities.device.unwrap();
        // Should contain 127.0.0.1 or detected IP
        assert!(device_caps.x_addr.contains("http://"));
    }
}
